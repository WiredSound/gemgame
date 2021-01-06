pub mod rendering;

use std::collections::{HashMap, HashSet};

use shared::{
    maps::{Chunk, ChunkCoords, Chunks, Map, Tile, TileCoords},
    messages
};

use crate::networking::{self, Connection, ConnectionTrait};

pub struct ClientMap {
    /// Chunks that are currently loaded (mapped to by chunk coordinate pairs).
    loaded_chunks: Chunks,
    /// Set of coordinate pairs for the chunks that are needed (i.e. chunks that are not already loaded but were needed
    /// to fulfill a call to [`chunk_at`] or [`tile_at`]). When a needed chunk is requested from the sever then its
    /// coordinates are added to the [`requested_chunks`] set. A chunks's coordinates are not removed from this set
    /// until the chunk itself is actually recevied.
    needed_chunks: HashSet<ChunkCoords>,
    /// Set of coordinate pairs for chunks that have been requested from the server but have not yet been received. A
    /// chunks's coordinates are remove from both this set and [`needed_chunks`] when the chunk itself is received from
    /// the server.
    requested_chunks: HashSet<ChunkCoords>
}

impl ClientMap {
    pub fn new() -> Self {
        ClientMap { loaded_chunks: HashMap::new(), needed_chunks: HashSet::new(), requested_chunks: HashSet::new() }
    }

    /// Attempt to get the tile at the specified tile coordinates.
    pub fn tile_at(&mut self, coords: TileCoords) -> Option<&Tile> {
        if !self.is_tile_loaded(coords) {
            let chunk_coords = coords.as_chunk_coords();
            let was_not_present = self.needed_chunks.insert(chunk_coords);

            if was_not_present {
                log::trace!(
                    "Added chunk at {} to list of needed chunks as it contained requested tile at {}",
                    chunk_coords,
                    coords
                );
            }
        }

        self.loaded_tile_at(coords)
    }

    pub fn chunk_at(&mut self, coords: ChunkCoords) -> Option<&Chunk> {
        if !self.is_chunk_loaded(coords) {
            let was_not_present = self.needed_chunks.insert(coords);

            if was_not_present {
                log::trace!("Added chunk at {} to list of needed chunks as it was requested", coords);
            }
        }

        self.loaded_chunk_at(coords)
    }

    pub fn request_needed_chunks_from_server(&mut self, ws: &mut Connection) -> networking::Result<()> {
        for coords in &self.needed_chunks {
            if !self.requested_chunks.contains(coords) {
                ws.send(&messages::ToServer::RequestChunk(*coords))?;
                self.requested_chunks.insert(*coords);
            }
        }

        Ok(())
    }
}

impl Map for ClientMap {
    fn loaded_chunk_at(&self, coords: ChunkCoords) -> Option<&Chunk> { self.loaded_chunks.get(&coords) }

    fn provide_chunk(&mut self, coords: ChunkCoords, chunk: Chunk) {
        // TODO: Unload chunk(s) should too many be loaded already?

        self.needed_chunks.remove(&coords);
        self.requested_chunks.remove(&coords);

        self.loaded_chunks.insert(coords, chunk);
    }
}