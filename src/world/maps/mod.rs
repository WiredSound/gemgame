use std::{ path::PathBuf, collections::HashMap };

use super::{ Coord, entities::Entity };

const CHUNK_WIDTH: Coord = 16;
const CHUNK_HEIGHT: Coord = 16;

pub struct Map {
    // Path to the directory containing map data.
    directory: PathBuf,

    /// The generator to be used when new chunks must be made.
    generator: Box<dyn Generator>,

    /// The currently loaded chunks that this map is comprised of mapped to
    /// chunk coordinates.
    loaded_chunks: HashMap<(Coord, Coord), Chunk>,

    /// Entities currently on this map.
    entities: Vec<Entity>
}

impl Map {
    fn new(directory: PathBuf, generator: Box<dyn Generator>) -> Self {
        Map {
            directory, generator,
            loaded_chunks: HashMap::new(),
            entities: Vec::new()
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
            log::debug!("Chunk ({}, {}) which contains tile at ({}, {}) is already loaded",
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
    /// chunk data could not be found (implies chunk has not yet been generated).
    fn load_chunk(&mut self, chunk_x: Coord, chunk_y: Coord) -> bool {
        // TODO: Read chunk data from file.

        false
    }

    fn unload_chunk(&mut self, chunk_x: Coord, chunk_y: Coord) {
        // TODO: Save chunk data to file.

        self.loaded_chunks.remove(&(chunk_x, chunk_y));
    }

    /// Will generate a new chunk at the given chunk coordinates using this map's
    /// generator. The newly generated chunk will be inserted into the
    /// `self.loaded_chunks` but will not be saved to file until it is unloaded
    /// (see [`Self::unload_chunk`]).
    fn generate_and_load_chunk(&mut self, chunk_x: Coord, chunk_y: Coord) {
        let chunk = self.generator.generate(chunk_x, chunk_y);
        self.loaded_chunks.insert((chunk_x, chunk_y), chunk);
    }

    fn get_loaded_chunk(&self, chunk_x: Coord, chunk_y: Coord) -> Option<&Chunk> {
        self.loaded_chunks.get(&(chunk_x, chunk_y))
    }
}

pub struct Chunk {
    /// The tiles that this chunk is comprised of.
    tiles: [Tile; (CHUNK_WIDTH * CHUNK_HEIGHT) as usize]
}

impl Chunk {
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

fn tile_coords_to_chunk_coords(x: Coord, y: Coord) -> (Coord, Coord) {
    let chunk_x = x / CHUNK_WIDTH;
    let chunk_y = y / CHUNK_HEIGHT;

    (
        if x >= 0 { chunk_x } else { chunk_x - 1 },
        if y >= 0 { chunk_y } else { chunk_y - 1 }
    )
}

fn tile_coords_to_chunk_offset_coords(x: Coord, y: Coord) -> (Coord, Coord) {
    let offset_x = x % CHUNK_WIDTH;
    let offset_y = y % CHUNK_HEIGHT;

    (
        if x >= 0 { offset_x } else { CHUNK_WIDTH + offset_x },
        if y >= 0 { offset_y } else { CHUNK_HEIGHT + offset_y }
    )
}

pub struct Tile {
    tile_type: TileType,
    blocking: bool
}

enum TileType {}

trait Generator {
    fn generate(&self, chunk_x: Coord, chunk_y: Coord) -> Chunk;
}

struct OverworldGenerator {}
//impl Generator for OverworldGenerator {}

#[cfg(test)]
mod test {
    #[test]
    fn tile_coords_to_chunk_coords() {
        assert_eq!(
            super::tile_coords_to_chunk_coords(0, 0),
            (0, 0)
        );

        assert_eq!(
            super::tile_coords_to_chunk_coords(8, 6),
            (0, 0)
        );

        assert_eq!(
            super::tile_coords_to_chunk_coords(12, -14),
            (0, -1)
        );

        assert_eq!(
            super::tile_coords_to_chunk_coords(-13, 14),
            (-1, 0)
        );

        assert_eq!(
            super::tile_coords_to_chunk_coords(-3, -2),
            (-1, -1)
        );

        assert_eq!(
            super::tile_coords_to_chunk_coords(-34, -19),
            (-3, -2)
        );
    }

    #[test]
    fn tile_coords_to_chunk_offset_coords() {
        assert_eq!(
            super::tile_coords_to_chunk_offset_coords(0, 0),
            (0, 0)
        );

        assert_eq!(
            super::tile_coords_to_chunk_offset_coords(8, 6),
            (8, 6)
        );

        assert_eq!(
            super::tile_coords_to_chunk_offset_coords(12, -14),
            (12, 2)
        );

        assert_eq!(
            super::tile_coords_to_chunk_offset_coords(-13, 14),
            (3, 14)
        );

        assert_eq!(
            super::tile_coords_to_chunk_offset_coords(-3, -2),
            (13, 14)
        );

        assert_eq!(
            super::tile_coords_to_chunk_offset_coords(-34, -19),
            (14, 13)
        );
    }
}