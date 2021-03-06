pub mod buttons;
pub mod menus;

pub use buttons::{Button, PurchaseButton, QuantityButton, SimpleButton};
use macroquad::prelude as quad;

const UI_TEXTURE_TILE_SIZE: u16 = 16;

fn calculate_draw_position(x: f32, y: f32, draw_width: f32, draw_height: f32) -> (f32, f32) {
    (
        (quad::screen_width() / 2.0) + (quad::screen_width() * x) - (draw_width / 2.0),
        (quad::screen_height() / 2.0) + (quad::screen_height() * y) - (draw_height / 2.0)
    )
}

fn calculate_draw_size(width: f32, height: f32) -> (f32, f32) {
    ((width * quad::screen_width()), (height * quad::screen_height()))
}

fn calculate_largest_squre_draw_size(size: f32) -> f32 {
    let (w, h) = calculate_draw_size(size, size);

    if (w - h).abs() > f32::EPSILON {
        w
    }
    else {
        h
    }
}
