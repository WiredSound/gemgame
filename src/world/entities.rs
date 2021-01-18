use std::{fmt, collections::HashMap};

use serde::{Deserialize, Serialize};

use super::maps::TileCoords;
use crate::Id;

/// Type alias for a hash map of entity IDs to entities.
pub type Entities = HashMap<Id, Entity>;

#[derive(Serialize, Deserialize, Clone)]
pub struct Entity {
    /// The name of this entity.
    pub name: String,
    /// The position of the entity within its current map.
    pub pos: TileCoords,
    /// The 'variety' of this entity (e.g. human, monster, etc.)
    pub variety: Variety
}

impl fmt::Display for Entity {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "'{}' at {} is a {}", self.name, self.pos, self.variety)
    }
}

#[derive(Serialize, Deserialize, Clone)]
pub enum Variety {
    Human {
        /// Direction that this human entity is facing. Defaults to 'down'.
        direction: Direction,
        /// Emotional expression of this entity (angry, shocked, etc.) Defaults to 'neutral'.
        facial_expression: FacialExpression,
        hair_style: HairStyle
    }
}

impl fmt::Display for Variety {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Variety::Human { direction, facial_expression, hair_style } => {
                write!(
                    f,
                    "human with hair style {} facing {} with {} facial expression",
                    hair_style, direction, facial_expression
                )
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum Direction {
    Up,
    Down,
    Left,
    Right
}

impl fmt::Display for Direction {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Direction::Up => write!(f, "🡑 up"),
            Direction::Down => write!(f, "🡓 down"),
            Direction::Left => write!(f, "🡐 left"),
            Direction::Right => write!(f, "🡒 right")
        }
    }
}

impl Default for Direction {
    fn default() -> Self { Direction::Down }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum HairStyle {
    Quiff,
    Mohawk,
    Fringe
}

impl fmt::Display for HairStyle {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            HairStyle::Quiff => write!(f, "short quiff"),
            HairStyle::Mohawk => write!(f, "edgy mohawk"),
            HairStyle::Fringe => write!(f, "simple fringe")
        }
    }
}

impl Default for HairStyle {
    fn default() -> Self { HairStyle::Quiff }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone, Copy, Hash)]
pub enum FacialExpression {
    /// Neutral 😐 facial expression.
    Neutral,
    /// Angry 😠 facial expression (both eyebrows slanted inward).
    Angry,
    /// Shocked/surprised 😲 facial expression (both eyebrows slanted outward, mouth opened wide).
    Shocked,
    /// Skeptical/suspicious 🤨 facial expression (single eyebrow slanted outward).
    Skeptical
}

impl fmt::Display for FacialExpression {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FacialExpression::Neutral => write!(f, "😐 neutral"),
            FacialExpression::Angry => write!(f, "😠 angry"),
            FacialExpression::Shocked => write!(f, "😲 shocked/surprised"),
            FacialExpression::Skeptical => write!(f, "🤨 skeptical/suspicious")
        }
    }
}

impl Default for FacialExpression {
    fn default() -> Self { FacialExpression::Neutral }
}
