pub mod generators;

use std::{
    path::{ Path, PathBuf },
    collections::HashMap,
    convert::TryInto,
    fs, fmt
};

use raylib::prelude::*;

use serde::{ Serialize, Deserialize };

use super::{ Coord, entities::Entity, load_json };
use crate::asset_management::Palette;

use generators::Generator;

const CHUNK_WIDTH: Coord = 16;
const CHUNK_HEIGHT: Coord = 16;
const CHUNK_TILE_COUNT: usize = (CHUNK_WIDTH * CHUNK_HEIGHT) as usize;

const MAP_JSON_FILE: &'static str = "map.json";

pub struct Map {
    /// Path to the directory containing map data.
    directory: PathBuf,

    /// The generator to be used when new chunks must be made.
    generator: Box<dyn Generator>,

    /// The currently loaded chunks that this map is comprised of (mapped to by
    /// chunk coordinates).
    loaded_chunks: HashMap<(Coord, Coord), Chunk>,

    /// Entities currently on this map.
    entities: Vec<Entity>
}

impl Map {
    /// Create a new map which will store its data to the specified directory
    /// and will be generated by the given generator.
    pub fn new(directory: PathBuf, generator: Box<dyn Generator>) -> Self {
        Map {
            directory, generator,
            loaded_chunks: HashMap::new(),
            entities: Vec::new()
        }
    }

    /// Attempt to load an existing map from the directory specified. This method
    /// relies on the [`load_json`] helper function.
    pub fn load(directory: PathBuf, seed: u32) -> Option<Self> {
        load_json("map", directory, MAP_JSON_FILE, |json, directory| {
            let generator_name = match json["generator"].as_str() {
                Some(value) => {
                    log::debug!("Generator name specified in JSON: {}", value);
                    value
                }
                None => {
                    log::warn!("Map '{}' does not have a generator specified - assuming 'surface' generator",
                                directory.display());
                    "surface"
                }
            };

            let generator = match generators::by_name(generator_name, seed) {
                Some(gen) => {
                    log::debug!("Generator specified: {}", gen.name());
                    gen
                }
                None => {
                    log::warn!("Map generator with name '{}' does not exist", generator_name);
                    Box::new(generators::SurfaceGenerator::new(seed))
                }
            };

            Map {
                directory, generator,
                loaded_chunks: HashMap::new(),
                entities: Vec::new()
            }
        })
    }

    /// Save this map to the filesystem. Will return `true` if able to save map
    /// data successfully as well as save all currently loaded map chunks.
    pub fn save(&self) -> bool {
        // Save currently loaded chunks:

        let mut chunks_saved_successfully = true;

        for ((chunk_x, chunk_y), chunk) in self.loaded_chunks.iter() {
            let success = chunk.save(&self.directory, *chunk_x, *chunk_y);
            chunks_saved_successfully = chunks_saved_successfully && success;
        }

        // Save map JSON data:

        let map_file_path = self.directory.join(MAP_JSON_FILE);

        let data = serde_json::json!({
            "generator": self.generator.name()
        }).to_string();

        match fs::write(&map_file_path, data) {
            Ok(_) => {
                log::info!("Saved map: {}", self);
                true && chunks_saved_successfully
            }

            Err(e) => {
                log::warn!("Failed to write map JSON file '{}' due to IO error: {}",
                           map_file_path.display(), e);
                false
            }
        }
    }

    /// Get a reference to the tile at the given coordinates. If the coordinates
    /// are for a tile in a chunk that has not been loaded, then it will be
    /// loaded. In the case of a chunk that has not yet been generated, it will
    /// be generated using this map's generator.
    pub fn tile_at(&mut self, x: Coord, y: Coord) -> &Tile {
        let chunk = self.chunk_at(x, y);

        let (offset_x, offset_y) = tile_coords_to_chunk_offset_coords(x, y);

        chunk.tile_at_offset(offset_x, offset_y)
    }

    /// Returns the map chunk at the given tile coordinates. If a chunk at those
    /// coordinates is not loaded, then the chunk will be read from disk. If
    /// chunk data does not exist then a new chunk is created.
    fn chunk_at(&mut self, x: Coord, y: Coord) -> &Chunk {
        let (chunk_x, chunk_y) = tile_coords_to_chunk_coords(x, y);

        if self.is_chunk_loaded(chunk_x, chunk_y) {
            log::trace!("Chunk ({}, {}) which contains tile at ({}, {}) is already loaded",
                        chunk_x, chunk_y, x, y);
        }
        else {
            if self.load_chunk(chunk_x, chunk_y) {
                log::debug!("Loaded chunk ({}, {}) as it contains requested tile ({}, {})",
                            chunk_x, chunk_y, x, y);
            }
            else {
                self.generate_and_load_chunk(chunk_x, chunk_y);
                log::info!("Generated chunk ({}, {})", chunk_x, chunk_y);
            }
        }

        self.get_loaded_chunk(chunk_x, chunk_y).unwrap()
    }

    /// Check if the chunk at the given chunk coordinates is loaded.
    fn is_chunk_loaded(&self, chunk_x: Coord, chunk_y: Coord) -> bool {
        self.loaded_chunks.contains_key(&(chunk_x, chunk_y))
    }

    /// Load the chunk at the given chunk coordinates by reading chunk data from
    /// the appropriate file. Will return `false` if the file containing the
    /// chunk data could not be found (suggests that that chunk has not yet been
    /// generated).
    fn load_chunk(&mut self, chunk_x: Coord, chunk_y: Coord) -> bool {
        if let Some(chunk) = Chunk::load(&self.directory, chunk_x, chunk_y) {
            self.loaded_chunks.insert((chunk_x, chunk_y), chunk);
            true
        } else { false }
    }

    /// Save to disk and remove from memory the chunk at the given chunk
    /// coordinates. If the specified chunk is not loaded then nothing will
    /// happen on call of this method.
    fn unload_chunk(&mut self, chunk_x: Coord, chunk_y: Coord) {
        if let Some(old_chunk) = self.loaded_chunks.remove(&(chunk_x, chunk_y)) {
            old_chunk.save(&self.directory, chunk_x, chunk_y);
        }
    }

    /// Will generate a new chunk at the given chunk coordinates using this map's
    /// generator. The newly generated chunk will be inserted into the
    /// [`Self::loaded_chunks`] but will not be saved to file until it is
    /// unloaded (see [`Self::unload_chunk`]).
    fn generate_and_load_chunk(&mut self, chunk_x: Coord, chunk_y: Coord) {
        let chunk = self.generator.generate(chunk_x, chunk_y);
        self.loaded_chunks.insert((chunk_x, chunk_y), chunk);
    }

    fn get_loaded_chunk(&self, chunk_x: Coord, chunk_y: Coord) -> Option<&Chunk> {
        self.loaded_chunks.get(&(chunk_x, chunk_y))
    }
}

impl fmt::Display for Map {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}' (generator: {}, loaded chunks: {})", self.directory.display(),
               self.generator.name(), self.loaded_chunks.len())
    }
}

/// 16x16 area of tiles on a map. As maps are infinite, chunks are generated,
/// loaded, and unloaded dynamically as necessary.
pub struct Chunk {
    /// The tiles that this chunk is comprised of.
    tiles: [Tile; CHUNK_TILE_COUNT]
}

impl Chunk {
    fn new(tiles: [Tile; CHUNK_TILE_COUNT]) -> Self {
        Chunk { tiles }
    }

    fn load(map_directory: &Path, chunk_x: Coord, chunk_y: Coord) -> Option<Self> {
        let chunk_file_path = map_directory.join(chunk_file_name(chunk_x, chunk_y));

        match fs::File::open(&chunk_file_path) {
            Ok(file) => {
                log::trace!("Opened chunk '{}' file: {:?}", chunk_file_path.display(), file);

                match bincode::deserialize_from::<fs::File, Vec<Tile>>(file) {
                    Ok(mut tiles_vec) => {
                        // TODO: May be able to skip the vector conversion when const generics are stablised?

                        tiles_vec.resize_with(CHUNK_TILE_COUNT, || {
                            log::warn!("Chunk '{}' contains the incorrect number of tiles",
                                       chunk_file_path.display());
                            Tile::default()
                        });

                        let tiles: [Tile; CHUNK_TILE_COUNT] = tiles_vec.try_into().unwrap();

                        log::debug!("Loaded chunk: {}", chunk_file_path.display());

                        Some(Chunk { tiles })
                    }

                    Err(e) => {
                        log::warn!("Chunk '{}' data could not be deserialised: {}",
                                   chunk_file_path.display(), e);
                        None
                    }
                }
            }

            Err(e) => {
                log::trace!("Could not open chunk '{}' file: {}",
                            chunk_file_path.display(), e);
                None
            }
        }
    }

    fn save(&self, map_directory: &Path, chunk_x: Coord, chunk_y: Coord) -> bool {
        let chunk_file_path = map_directory.join(chunk_file_name(chunk_x, chunk_y));

        match fs::File::create(&chunk_file_path) {
            Ok(file) => {
                // TODO: Const generics?
                let tiles_vec = self.tiles.to_vec();

                if let Err(e) = bincode::serialize_into(file, &tiles_vec) {
                    log::warn!("Failed to serialize and write chunk '{}': {}",
                               chunk_file_path.display(), e);

                    return false;
                }

                log::debug!("Saved chunk: {}", chunk_file_path.display());
            }

            Err(e) => {
                log::trace!("Could not write chunk '{}' file: {}",
                            chunk_file_path.display(), e);

                return false;
            }
        }

        true
    }

    fn tile_at_offset(&self, mut x: Coord, mut y: Coord) -> &Tile {
        if x < 0 || x >= CHUNK_WIDTH {
            log::warn!("Chunk x-offset is out of bounds: {}", x);
            x = 0;
        }
        if y < 0 || y >= CHUNK_HEIGHT {
            log::warn!("Chunk y-offset is out of bounds: {}", y);
            y = 0;
        }

        &self.tiles[(y * CHUNK_WIDTH + x) as usize]
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Tile {
    /// Indicates characteristics of this tile such as its texture.
    tile_type: TileType,
    /// Whether or not this tile has been seen by the player yet.
    seen: bool
}

impl Tile {
    fn default() -> Self {
        Tile { tile_type: TileType::Ground, seen: false }
    }

    pub const fn texture_rec(&self, individual_tile_size: i32) -> Rectangle {
        let (offset_x, offset_y) = match self.tile_type {
            TileType::Ground => (0, 0),
            TileType::Wall => (1, 0),
            TileType::Dirt => (2, 0),
            TileType::Flower(_) => (0, 1),
            TileType::Tree(_) => (1, 1),
            TileType::Bush(_) => (2, 1)
        };

        let texture_x = offset_x * individual_tile_size;
        let texture_y = offset_y * individual_tile_size;

        Rectangle::new(texture_x as f32, texture_y as f32,
                       individual_tile_size as f32, individual_tile_size as f32)
    }

    pub const fn texture_col(&self, colours: &Palette) -> Color {
        match &self.tile_type {
            TileType::Ground
            | TileType::Dirt => colours.ground,

            TileType::Wall => colours.wall,

            TileType::Flower(state)
            | TileType::Tree(state)
            | TileType::Bush(state) => match &state {
                PlantState::Ripe => colours.ripe_plant,
                PlantState::Harvested => colours.harvested_plant,
                PlantState::Dead => colours.dead_plant
            }
        }
    }

    pub const fn blocking(&self) -> bool {
        match self.tile_type {
            TileType::Wall
            | TileType::Flower(_) | TileType::Tree(_) | TileType::Bush(_) => true,

            _ => false
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TileType {
    Ground, Wall, Dirt,
    Flower(PlantState), Tree(PlantState), Bush(PlantState)
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PlantState { Ripe, Harvested, Dead }

const fn tile_coords_to_chunk_coords(x: Coord, y: Coord) -> (Coord, Coord) {
    let chunk_x = x / CHUNK_WIDTH;
    let chunk_y = y / CHUNK_HEIGHT;
    (
        if x >= 0 || x % CHUNK_WIDTH == 0 { chunk_x } else { chunk_x - 1 },
        if y >= 0 || y % CHUNK_HEIGHT == 0 { chunk_y } else { chunk_y - 1 }
    )
}

const fn tile_coords_to_chunk_offset_coords(x: Coord, y: Coord) -> (Coord, Coord) {
    let offset_x = x % CHUNK_WIDTH;
    let offset_y = y % CHUNK_HEIGHT;
    (
        if x >= 0 || offset_x == 0 { offset_x } else { CHUNK_WIDTH + offset_x },
        if y >= 0 || offset_y == 0 { offset_y } else { CHUNK_HEIGHT + offset_y }
    )
}

fn chunk_file_name(chunk_x: Coord, chunk_y: Coord) -> String {
    format!("{}_{}.chunk", chunk_x, chunk_y)
}

#[cfg(test)]
mod test {
    #[test]
    fn tile_coords_to_chunk_coords() {
        let test_data = &[
            ((0, 0), (0, 0)),
            ((12, -14), (0, -1)),
            ((-14, 14), (-1, 0)),
            ((-3, -2), (-1, -1)),
            ((-34, -19), (-3, -2)),
            ((-16, -17), (-1, -2)),
            ((-33, -32), (-3, -2))
        ];
        for ((in_x, in_y), out) in test_data {
            assert_eq!(super::tile_coords_to_chunk_coords(*in_x, *in_y), *out);
        }
    }

    #[test]
    fn tile_coords_to_chunk_offset_coords() {
        let test_data = &[
            ((0, 0), (0, 0)),
            ((8, 6), (8, 6)),
            ((12, -14), (12, 2)),
            ((-13, 14), (3, 14)),
            ((-3, -2), (13, 14)),
            ((-34, -19), (14, 13)),
            ((-16, -17), (0, 15)),
            ((-33, -32), (15, 0))
        ];
        for ((in_x, in_y), out) in test_data {
            assert_eq!(super::tile_coords_to_chunk_offset_coords(*in_x, *in_y), *out);
        }
    }

    #[test]
    fn chunk_file_name() {
        assert_eq!(&super::chunk_file_name(0, 0), "0_0.chunk");
        assert_eq!(&super::chunk_file_name(2, -11), "2_-11.chunk");
    }
}