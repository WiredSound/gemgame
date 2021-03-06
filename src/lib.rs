pub mod gems;
pub mod id;
pub mod items;
pub mod maps;
pub mod messages;

pub use id::Id;

/// Version of this client/server build.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");
