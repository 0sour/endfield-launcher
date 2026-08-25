pub mod consts;
pub mod crypto;

pub mod api;
mod game;
#[cfg(feature = "install")]
pub mod payload;
#[cfg(feature = "install")]
pub mod repairer;
#[cfg(feature = "install")]
pub mod version_diff;

pub use game::Game;

pub mod prelude {
    pub use super::consts::*;
    pub use super::game::Game;
    #[cfg(feature = "install")]
    pub use super::payload;
    #[cfg(feature = "install")]
    pub use super::repairer;
    #[cfg(feature = "install")]
    pub use super::version_diff::*;
}
