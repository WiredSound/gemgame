use macroquad::prelude as quad;
use shared::gems::{self, Gem};

use crate::{AssetManager, TextureKey};

const GEM_COLLECTION_TEXTURE_SOURCE: quad::Rect =
    crate::make_texture_source_rect(super::UI_TEXTURE_TILE_SIZE, (0, 3), (2, 3));

pub fn draw_gem_collection_menu(x: f32, y: f32, width: f32, gem_collection: &gems::Collection, assets: &AssetManager) {
    let draw_width = quad::screen_width() * width;
    let draw_height = draw_width * 1.5;

    let (draw_x, draw_y) = super::calculate_draw_position(x, y, draw_width, draw_height);

    quad::draw_texture_ex(
        assets.texture(TextureKey::Ui),
        draw_x,
        draw_y,
        quad::WHITE,
        quad::DrawTextureParams {
            dest_size: Some(quad::vec2(draw_width, draw_height)),
            source: Some(GEM_COLLECTION_TEXTURE_SOURCE),
            ..Default::default()
        }
    );

    for (gem, offset) in &[(Gem::Emerald, -0.25), (Gem::Ruby, 0.0), (Gem::Diamond, 0.25)] {
        quad::draw_text(
            &format!("{:2}", gem_collection.get_quantity(*gem)),
            draw_x + (draw_width * 0.6),
            draw_y + (draw_height * (0.53 + offset)),
            draw_width * 0.2,
            quad::GRAY
        );
    }
}

// pub fn draw_leaderboard_menu
