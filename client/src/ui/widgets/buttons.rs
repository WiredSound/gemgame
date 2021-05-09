use macroquad::prelude as quad;

use crate::{AssetManager, TextureKey};

const BUTTON_TEXTURE_TILE_SIZE: u16 = 32;

const BUTTON_UP_RELATIVE_TEXTURE_COORDS: (u16, u16) = (0, 0);
const BUTTON_DOWN_RELATIVE_TEXTURE_COORDS: (u16, u16) = (1, 0);

const INTERACT_SIZE_MULTIPLIER: f32 = 0.6;

const NOT_HOVER_COLOUR: quad::Color = quad::Color::new(0.8, 0.8, 0.8, 1.0);
const HOVER_COLOUR: quad::Color = quad::WHITE;

pub fn make_open_purchase_menu_button(x: f32, y: f32) -> SimpleButton {
    SimpleButton { is_hover: false, is_down: false, x, y, icon_texture_x: 0, icon_texture_y: 1 }
}

pub fn make_place_bomb_button(x: f32, y: f32) -> QuantityButton {
    QuantityButton {
        button: SimpleButton { is_hover: false, is_down: false, x, y, icon_texture_x: 0, icon_texture_y: 2 },
        quantity: 3,
        quantity_bars_texture_x: 1,
        quantity_bars_texture_y: 2
    }
}

pub fn make_purchase_button() -> SimpleButton {
    unimplemented!()
}

pub trait Button {
    /// Determines whether the button is being hovered over and/or pressed based on mouse position & whether or not the
    /// left mouse button is down. Returns true once when the button is clicked on.
    fn update(&mut self, size: f32) -> bool;

    /// Draws the button to the screen. Should return the absolute position (first pair of values in returned tuple) and
    /// size (second tuple value) that button was drawn.
    fn draw(&self, assets: &AssetManager, size: f32) -> ((f32, f32), f32);
}

pub struct SimpleButton {
    is_hover: bool,
    is_down: bool,
    x: f32,
    y: f32,
    icon_texture_x: u16,
    icon_texture_y: u16
}

impl Button for SimpleButton {
    fn update(&mut self, size: f32) -> bool {
        let (mouse_x, mouse_y) = quad::mouse_position();

        let draw_size = super::calculate_largest_squre_draw_size(size) * INTERACT_SIZE_MULTIPLIER;
        let (draw_x, draw_y) = super::calculate_draw_position(self.x, self.y, draw_size, draw_size);

        let rect = quad::Rect { x: draw_x, y: draw_y, w: draw_size, h: draw_size };

        let was_down = self.is_down;

        self.is_hover = rect.contains(quad::vec2(mouse_x, mouse_y));
        self.is_down = self.is_hover && quad::is_mouse_button_down(quad::MouseButton::Left);

        !was_down && self.is_down
    }

    fn draw(&self, assets: &AssetManager, size: f32) -> ((f32, f32), f32) {
        // Button sizes are calculated as a fraction of either the screen width or height (whichever is larger).

        let draw_size = super::calculate_largest_squre_draw_size(size);
        let dest_size = Some(quad::vec2(draw_size, draw_size));

        // Positions of the buttons are expressed relative to the screen size with each coordinate being within the -0.5
        // to 0.5 range.

        let (draw_x, draw_y) = super::calculate_draw_position(self.x, self.y, draw_size, draw_size);

        quad::draw_texture_ex(
            assets.texture(TextureKey::Ui),
            draw_x,
            draw_y,
            if self.is_hover { HOVER_COLOUR } else { NOT_HOVER_COLOUR },
            quad::DrawTextureParams {
                dest_size,
                source: Some(crate::make_texture_source_rect(
                    BUTTON_TEXTURE_TILE_SIZE,
                    if self.is_down { BUTTON_DOWN_RELATIVE_TEXTURE_COORDS } else { BUTTON_UP_RELATIVE_TEXTURE_COORDS }
                )),
                ..Default::default()
            }
        );

        quad::draw_texture_ex(
            assets.texture(TextureKey::Ui),
            draw_x,
            draw_y,
            quad::WHITE,
            quad::DrawTextureParams {
                dest_size,
                source: Some(crate::make_texture_source_rect(
                    BUTTON_TEXTURE_TILE_SIZE,
                    (self.icon_texture_x, self.icon_texture_y)
                )),
                ..Default::default()
            }
        );

        ((draw_x, draw_y), draw_size)
    }
}

pub struct QuantityButton {
    button: SimpleButton,
    quantity: u32,
    quantity_bars_texture_x: u16,
    quantity_bars_texture_y: u16
}

impl Button for QuantityButton {
    fn update(&mut self, size: f32) -> bool {
        self.button.update(size)
    }

    fn draw(&self, assets: &AssetManager, size: f32) -> ((f32, f32), f32) {
        let ((draw_x, draw_y), draw_size) = self.button.draw(assets, size);

        if self.quantity > 0 {
            let bar_texture_offset = std::cmp::min(self.quantity as u16 - 1, 3);

            let quarter_texture_tile_size = BUTTON_TEXTURE_TILE_SIZE / 4;

            quad::draw_texture_ex(
                assets.texture(TextureKey::Ui),
                draw_x + (draw_size / 4.0),
                draw_y,
                quad::WHITE,
                quad::DrawTextureParams {
                    dest_size: Some(quad::vec2(draw_size / 4.0, draw_size)),
                    source: Some(quad::Rect {
                        x: ((self.quantity_bars_texture_x * BUTTON_TEXTURE_TILE_SIZE)
                            + (bar_texture_offset * quarter_texture_tile_size)) as f32,
                        y: (self.quantity_bars_texture_y * BUTTON_TEXTURE_TILE_SIZE) as f32,
                        w: quarter_texture_tile_size as f32,
                        h: BUTTON_TEXTURE_TILE_SIZE as f32
                    }),
                    ..Default::default()
                }
            );
        }

        ((draw_x, draw_y), draw_size)
    }
}