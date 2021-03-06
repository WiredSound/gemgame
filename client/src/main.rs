mod asset_management;
mod maps;
mod networking;
mod sessions;
mod states;
mod ui;

use macroquad::prelude as quad;

#[cfg(debug_assertions)]
const CONNECTION_STR: &str = "ws://localhost:5678";
#[cfg(not(debug_assertions))]
const CONNECTION_STR: &str = "wss://gemgame.mblack.dev/wss";

#[macroquad::main("Client")]
async fn main() {
    #[cfg(not(target_arch = "wasm32"))]
    pretty_env_logger::init();

    let mut assets = AssetManager::new("assets/", "textures/");

    log::info!("Prepared the asset manager");

    let mut current_state: Box<dyn states::State> = Box::new(states::pregame::ConnectingState::new(CONNECTION_STR));
    assets.required_textures(current_state.required_textures()).await;

    log::info!("Created initial state '{}' - beginning main loop...", current_state.title());

    loop {
        // Update game logic and draw:

        quad::clear_background(quad::BLACK);

        let delta = quad::get_frame_time();
        let potential_state_change = current_state.update_and_draw(&assets, delta);

        quad::next_frame().await;

        // Handle state transition (if necessary):

        if let Some(next_state) = potential_state_change {
            assets.required_textures(next_state.required_textures()).await;
            log::info!("Changing state from '{}' to '{}'", current_state.title(), next_state.title());
            current_state = next_state;
        }
    }
}

pub type AssetManager = asset_management::AssetManager<TextureKey>;

#[derive(Debug, Hash, PartialEq, Eq, Clone, Copy)]
pub enum TextureKey {
    Tiles,
    Entities,
    Bombs,
    Ui
}

impl asset_management::AssetKey for TextureKey {
    fn path(&self) -> &str {
        match self {
            TextureKey::Tiles => "tileset.png",
            TextureKey::Entities => "entities.png",
            TextureKey::Bombs => "bombs.png",
            TextureKey::Ui => "ui.png"
        }
    }
}

const fn make_texture_source_rect(
    single_tile_texture_size: u16, relative_texture_coords: (u16, u16), relative_texture_size: (u16, u16)
) -> quad::Rect {
    let (relative_x, relative_y) = relative_texture_coords;
    let (relative_width, relative_height) = relative_texture_size;

    quad::Rect {
        x: (relative_x * single_tile_texture_size) as f32,
        y: (relative_y * single_tile_texture_size) as f32,
        w: (relative_width * single_tile_texture_size) as f32,
        h: (relative_height * single_tile_texture_size) as f32
    }
}
