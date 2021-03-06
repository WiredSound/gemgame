use std::collections::HashMap;

use shared::maps::{Chunk, OffsetCoords, Tile, CHUNK_HEIGHT, CHUNK_WIDTH};

#[derive(Default)]
pub struct ChunkPlan {
    tile_categories: HashMap<(i32, i32), TileCategory>
}

impl ChunkPlan {
    pub fn set_category_at(&mut self, offset_x: i32, offset_y: i32, category: TileCategory) {
        self.tile_categories.insert((offset_x, offset_y), category);
    }

    pub fn to_chunk(
        &self, dirt_transitions: &TransitionTiles, water_transitions: &TransitionTiles,
        mut place_non_transition_tile: impl FnMut(TileCategory, i32, i32) -> Tile
    ) -> Chunk {
        let mut chunk = Chunk::default();

        for offset_x in 0..CHUNK_WIDTH {
            for offset_y in 0..CHUNK_HEIGHT {
                let category = self.get_category_at(offset_x, offset_y);

                let tile = self
                    .maybe_transition_tile(offset_x, offset_y, dirt_transitions, water_transitions)
                    .unwrap_or_else(|| place_non_transition_tile(category, offset_x, offset_y));

                let coords = OffsetCoords { x: offset_x as u8, y: offset_y as u8 };
                chunk.set_tile_at_offset(coords, tile);
            }
        }

        chunk
    }

    pub fn remove_all_juttting_and_unconnected_tiles(&mut self) {
        for offset_x in 0..CHUNK_WIDTH {
            for offset_y in 0..CHUNK_HEIGHT {
                self.remove_juttting_and_unconnected_tiles_at(offset_x, offset_y);
            }
        }
    }

    fn remove_juttting_and_unconnected_tiles_at(&mut self, offset_x: i32, offset_y: i32) {
        let category = self.get_category_at(offset_x, offset_y);

        let (above, below, left, right) = self.surrounding_not_equal_to(category, offset_x, offset_y);

        if (above && below && left) || (above && below && right) || (above && left && right) || (below && left && right)
        {
            self.set_category_at(offset_x, offset_y, TileCategory::default());

            if !above {
                self.remove_juttting_and_unconnected_tiles_at(offset_x, offset_y + 1);
            }
            if !below {
                self.remove_juttting_and_unconnected_tiles_at(offset_x, offset_y - 1);
            }
            if left {
                self.remove_juttting_and_unconnected_tiles_at(offset_x - 1, offset_y);
            }
            if !right {
                self.remove_juttting_and_unconnected_tiles_at(offset_x + 1, offset_y);
            }
        }
    }

    fn maybe_transition_tile(
        &self, offset_x: i32, offset_y: i32, dirt_transitions: &TransitionTiles, water_transitions: &TransitionTiles
    ) -> Option<Tile> {
        let my_category = self.get_category_at(offset_x, offset_y);

        let my_transition_tiles = {
            match my_category {
                TileCategory::Water => water_transitions,
                TileCategory::Dirt => dirt_transitions,
                TileCategory::Grass => return None
            }
        };

        let transition_tile = match self.surrounding_not_equal_to(my_category, offset_x, offset_y) {
            // Right-angle transition tiles:
            (true, _, true, false) => Some(my_transition_tiles.top_left),
            (true, _, false, true) => Some(my_transition_tiles.top_right),
            (_, true, true, false) => Some(my_transition_tiles.bottom_left),
            (_, true, false, true) => Some(my_transition_tiles.bottom_right),

            // Straight transition tiles:
            (true, _, false, false) => Some(my_transition_tiles.top),
            (_, true, false, false) => Some(my_transition_tiles.bottom),
            (false, false, true, _) => Some(my_transition_tiles.left),
            (false, false, _, true) => Some(my_transition_tiles.right),

            _ => None
        };

        transition_tile.or_else(|| {
            let top_left = self.get_category_at(offset_x - 1, offset_y + 1) != my_category;
            let top_right = self.get_category_at(offset_x + 1, offset_y + 1) != my_category;
            let bottom_left = self.get_category_at(offset_x - 1, offset_y - 1) != my_category;
            let bottom_right = self.get_category_at(offset_x + 1, offset_y - 1) != my_category;

            match (top_left, top_right, bottom_left, bottom_right) {
                // Corner tile transitions:
                (true, false, false, _) => Some(my_transition_tiles.corner_top_left),
                (false, true, _, false) => Some(my_transition_tiles.corner_top_right),
                (false, _, true, false) => Some(my_transition_tiles.corner_bottom_left),
                (_, false, false, true) => Some(my_transition_tiles.corner_bottom_right),

                _ => None
            }
        })
    }

    fn get_category_at(&self, offset_x: i32, offset_y: i32) -> TileCategory {
        *self.tile_categories.get(&(offset_x, offset_y)).unwrap_or(&TileCategory::default())
    }

    fn surrounding_not_equal_to(
        &self, category: TileCategory, offset_x: i32, offset_y: i32
    ) -> (bool, bool, bool, bool) {
        (
            self.get_category_at(offset_x, offset_y + 1) != category, // above
            self.get_category_at(offset_x, offset_y - 1) != category, // below
            self.get_category_at(offset_x - 1, offset_y) != category, // left
            self.get_category_at(offset_x + 1, offset_y) != category  // right
        )
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TileCategory {
    Grass,
    Dirt,
    Water
}

impl Default for TileCategory {
    fn default() -> Self {
        TileCategory::Grass
    }
}

pub struct TransitionTiles {
    pub top: Tile,
    pub bottom: Tile,
    pub left: Tile,
    pub right: Tile,
    pub top_left: Tile,
    pub top_right: Tile,
    pub bottom_left: Tile,
    pub bottom_right: Tile,
    pub corner_top_left: Tile,
    pub corner_top_right: Tile,
    pub corner_bottom_left: Tile,
    pub corner_bottom_right: Tile
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_chunk_plan(
        area_width: i32, area_height: i32, dirt_positions_before: Vec<(i32, i32)>,
        dirt_positions_after: Vec<(i32, i32)>
    ) {
        let mut chunk = ChunkPlan::default();

        for x in 0..area_width {
            for y in 0..area_height {
                let category =
                    if dirt_positions_before.contains(&(x, y)) { TileCategory::Dirt } else { TileCategory::Grass };
                chunk.set_category_at(x, y, category);
            }
        }

        chunk.remove_all_juttting_and_unconnected_tiles();

        for x in 0..area_width {
            for y in 0..area_height {
                assert_eq!(
                    chunk.get_category_at(x as i32, y as i32),
                    if dirt_positions_after.contains(&(x, y)) { TileCategory::Dirt } else { TileCategory::Grass }
                );
            }
        }
    }

    #[test]
    #[rustfmt::skip]
    fn remove_jutting_tiles() {
        // .....      .....
        // ..##.  ->  ..##.
        // .###.      ..##.
        // .....      .....

        test_chunk_plan(
            5, 4,
            vec![(2, 1), (3, 1), (1, 2), (2, 2), (3, 2)],
            vec![(2, 1), (3, 1), (2, 2), (3, 2)]
        );

        // ......      ......
        // .####.  ->  ...##.
        // ...##.      ...##.
        // ......      ......

        test_chunk_plan(
            6, 4,
            vec![(1, 1), (2, 1), (3, 1), (4, 1), (3, 2), (4, 2)],
            vec![(3, 1), (4, 1), (3, 2), (4, 2)]
        );

        // .....
        // ..##.
        // .###.  ->  no change
        // .###.
        // ..##.
        // .....

        let dirt_positions = vec![(2, 1), (3, 1), (1, 2), (2, 2), (3, 2), (1, 3), (2, 3), (3, 3), (2, 4), (3, 4)];
        test_chunk_plan(
            5, 6,
            dirt_positions.clone(),
            dirt_positions
        );
    }

    #[test]
    #[rustfmt::skip]
    fn remove_unconnected_tiles() {
        // Not allowed:

        // ...
        // .#.
        // ...

        test_chunk_plan(
            3, 3,
            vec![(1, 1)], vec![]
        );

        // ....
        // .##.
        // ....

        test_chunk_plan(
            4, 3,
            vec![(1, 1), (2, 1)], vec![]
        );

        // ....
        // .##.
        // .#..
        // ....

        test_chunk_plan(
            4, 4,
            vec![(1, 1), (2, 1), (1, 2)], vec![]
        );

        // Allowed:

        // ....
        // .##.
        // .##.
        // ....

        let dirt_positions = vec![(1, 1), (2, 1), (1, 2), (2, 2)];
        test_chunk_plan(
            4, 4,
            dirt_positions.clone(),
            dirt_positions
        );
    }
}
