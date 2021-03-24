mod entities;
mod tiles;

use std::collections::HashMap;

use macroquad::prelude as quad;
use shared::{
    maps::{entities::Entity, Map, TileCoords},
    Id
};

use crate::{maps::ClientMap, AssetManager, TextureKey};

const ENTITY_POSITION_CORRECTED_MOVEMENT_TIME: f32 = 0.025;

/// Handles the drawing of a game map.
pub struct Renderer {
    /// The camera context in which the map will be rendered.
    camera: quad::Camera2D,
    /// The width and height (in camera space) that each tile will be draw as.
    tile_draw_size: f32,
    /// The width and height (in pixels) that each individual tile on the tiles texture is.
    tile_texture_size: u16,
    my_entity_renderer: entities::Renderer,
    remote_entity_renderers: HashMap<Id, entities::Renderer>
}

impl Renderer {
    pub fn new(tile_draw_size: f32, tile_texture_size: u16, my_entity_pos: TileCoords) -> Self {
        Renderer {
            camera: quad::Camera2D::default(),
            tile_draw_size,
            tile_texture_size,
            my_entity_renderer: entities::Renderer::new(my_entity_pos, tile_draw_size),
            remote_entity_renderers: HashMap::new()
        }
    }

    /// Draws the tiles & entities than are within the bounds of the camera's viewport.
    pub fn draw(&mut self, map: &ClientMap, my_entity_contained: &Entity, assets: &AssetManager, delta: f32) {
        // Adjust camera zoom so that textures don't become distorted when the screen is resized:

        self.camera.zoom = {
            if quad::screen_width() > quad::screen_height() {
                quad::vec2(1.0, quad::screen_width() / quad::screen_height())
            }
            else {
                quad::vec2(quad::screen_height() / quad::screen_width(), 1.0)
            }
        };

        // Update this client's entity and centre camera around it:

        self.my_entity_renderer.update(delta);
        self.camera.target = self.my_entity_renderer.current_pos;

        // Begin drawing in camera space:
        quad::set_camera(self.camera);

        // Establish the tile area of the map that is actually on-screen:

        let on_screen_tiles_left_boundary = ((self.camera.target.x - 1.0) / self.tile_draw_size).floor() as i32;
        let on_screen_tiles_right_boundary = ((self.camera.target.x + 1.0) / self.tile_draw_size).ceil() as i32;
        let on_screen_tiles_bottom_boundary = ((self.camera.target.y - 1.0) / self.tile_draw_size).floor() as i32;
        let on_screen_tiles_top_boundary = ((self.camera.target.y + 1.0) / self.tile_draw_size).ceil() as i32;

        // Draw tiles:

        let mut draw_pos;
        for tile_x in on_screen_tiles_left_boundary..on_screen_tiles_right_boundary {
            for tile_y in on_screen_tiles_bottom_boundary..on_screen_tiles_top_boundary {
                draw_pos = tile_coords_to_vec2(TileCoords { x: tile_x, y: tile_y }, self.tile_draw_size);

                // If the tile at the specified coordinates is in a chunk that is already loaded then it will be drawn.
                // Otherwise, a grey placeholder rectangle will be drawn in its place until the required chunk is
                // received from the server.

                if let Some(tile) = map.loaded_tile_at(TileCoords { x: tile_x, y: tile_y }) {
                    tiles::draw(
                        tile,
                        draw_pos,
                        self.tile_draw_size,
                        self.tile_texture_size,
                        assets.texture(TextureKey::Tiles)
                    );
                }
                else {
                    tiles::draw_pending_tile(draw_pos, self.tile_draw_size);
                }
            }
        }

        // Update remote entities:

        for renderer in self.remote_entity_renderers.values_mut() {
            renderer.update(delta);
        }

        // Draw remote entities:

        let remote_entities_to_draw: Vec<(&Entity, &entities::Renderer)> = self
            .remote_entity_renderers
            .iter()
            .filter_map(|(id, renderer)| {
                if let Some(entity) = map.entity_by_id(*id) {
                    // Is the entity actually on screen?
                    if on_screen_tiles_left_boundary <= entity.pos.x
                        && entity.pos.x <= on_screen_tiles_right_boundary
                        && on_screen_tiles_bottom_boundary <= entity.pos.y
                        && entity.pos.y <= on_screen_tiles_top_boundary
                    {
                        return Some((entity, renderer));
                    }
                }
                None
            })
            .collect();

        // Draw lower portion of each on-screen entity:
        for (entity, renderer) in &remote_entities_to_draw {
            renderer.draw_lower(
                entity,
                assets.texture(TextureKey::Entities),
                self.tile_draw_size,
                self.tile_texture_size
            );
        }
        // Draw upper portion of each on-screen entity:
        for (entity, renderer) in &remote_entities_to_draw {
            renderer.draw_upper(
                entity,
                assets.texture(TextureKey::Entities),
                self.tile_draw_size,
                self.tile_texture_size
            );
        }

        // Draw this client's entity:

        self.my_entity_renderer.draw_lower(
            my_entity_contained,
            assets.texture(TextureKey::Entities),
            self.tile_draw_size,
            self.tile_texture_size
        );

        self.my_entity_renderer.draw_upper(
            my_entity_contained,
            assets.texture(TextureKey::Entities),
            self.tile_draw_size,
            self.tile_texture_size
        )
    }

    /// Begin the animated movement of this client's player entity to the specified position. This method is to be
    /// called by the [`crate::maps::entities::MyEntity::move_towards_checked`] method.
    pub fn my_entity_moved(&mut self, from_coords: TileCoords, to_coords: TileCoords, movement_time: f32) {
        self.my_entity_renderer.do_movement(from_coords, to_coords, movement_time, self.tile_draw_size);
    }

    /// Begin a shorter animation of this client's entity to the specified position. This method is to be called by the
    /// [`crate::maps::entities::MyEntity::received_movement_reconciliation'] method.
    pub fn my_entity_position_corrected(&mut self, incorrect_coords: TileCoords, correct_coords: TileCoords) {
        self.my_entity_renderer.do_movement(
            incorrect_coords,
            correct_coords,
            ENTITY_POSITION_CORRECTED_MOVEMENT_TIME,
            self.tile_draw_size
        );
    }

    /// Begin the animated movement of the specified remote entity to the given position. This method is to be called by
    /// the [`ClientMap::set_remote_entity_position`].
    pub fn remote_entity_moved(
        &mut self, entity_id: Id, from_coords: TileCoords, to_coords: TileCoords, movement_time: f32
    ) {
        self.remote_entity_renderers.entry(entity_id).or_default().do_movement(
            from_coords,
            to_coords,
            movement_time,
            self.tile_draw_size
        );
    }

    pub fn add_remote_entity(&mut self, entity_id: Id, coords: TileCoords) {
        self.remote_entity_renderers.insert(entity_id, entities::Renderer::new(coords, self.tile_draw_size));
    }

    pub fn remove_remote_entity(&mut self, entity_id: Id) {
        self.remote_entity_renderers.remove(&entity_id);
    }
}

fn tile_coords_to_vec2(coords: TileCoords, tile_draw_size: f32) -> quad::Vec2 {
    quad::vec2(coords.x as f32 * tile_draw_size, coords.y as f32 * tile_draw_size)
}