pub mod entities;
pub mod rendering;

use std::collections::{HashMap, HashSet};

use shared::{
    maps::{
        entities::{Entities, Entity},
        Chunk, ChunkCoords, Chunks, Map, Tile, TileCoords
    },
    messages, Id
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
    requested_chunks: HashSet<ChunkCoords>,
    /// All entities (except this client's player entity) that are on this map and within currently loaded chunks.
    entities: Entities
}

impl ClientMap {
    pub fn new() -> Self {
        ClientMap {
            loaded_chunks: HashMap::new(),
            needed_chunks: HashSet::new(),
            requested_chunks: HashSet::new(),
            entities: HashMap::new()
        }
    }

    /// Attempt to get the tile at the specified tile coordinates.
    /// TODO: Remove this method, have server automatically send chunks to client based on player position.
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

    /// TODO: Remove this method, reason as above.
    pub fn request_needed_chunks_from_server(&mut self, ws: &mut Connection) -> networking::Result<()> {
        for coords in &self.needed_chunks {
            if !self.requested_chunks.contains(coords) {
                ws.send(&messages::ToServer::RequestChunk(*coords))?;
                self.requested_chunks.insert(*coords);
            }
        }

        Ok(())
    }

    pub fn is_position_free(&mut self, coords: TileCoords) -> bool {
        let tile_blocking = self.tile_at(coords).map_or(true, |tile| tile.is_blocking());

        if tile_blocking {
            false
        }
        else {
            // Determining if there are blocking entities like this is O(n) so may need a better solution for instances
            // where many entities are together in a small area (e.g. like the O(1) solution seen on server side).

            let entity_blocking = self.entities.values().any(|entity| entity.pos == coords);
            !entity_blocking
        }
    }

    pub fn set_entity_position_by_id(&mut self, id: Id, new_pos: TileCoords) {
        if let Some(entity) = self.entities.get_mut(&id) {
            entity.pos = new_pos;
        }
        else {
            log::warn!("Cannot set position of entity {} as it is not loaded", id);
        }
    }
}

impl Map for ClientMap {
    fn loaded_chunk_at(&self, coords: ChunkCoords) -> Option<&Chunk> {
        self.loaded_chunks.get(&coords)
    }

    fn loaded_chunk_at_mut(&mut self, coords: ChunkCoords) -> Option<&mut Chunk> {
        self.loaded_chunks.get_mut(&coords)
    }

    fn provide_chunk(&mut self, coords: ChunkCoords, chunk: Chunk) {
        // TODO: Unload chunk(s) should too many be loaded already?

        self.needed_chunks.remove(&coords);
        self.requested_chunks.remove(&coords);

        self.loaded_chunks.insert(coords, chunk);
    }

    fn entity_by_id(&self, id: Id) -> Option<&Entity> {
        self.entities.get(&id)
    }

    fn add_entity(&mut self, id: Id, entity: Entity) {
        self.entities.insert(id, entity);
        log::info!("Entity with ID {} added to game map", id);
    }

    fn remove_entity(&mut self, id: Id) -> Option<Entity> {
        log::info!("Removing entity with ID {} from game map", id);
        self.entities.remove(&id)
    }
}
