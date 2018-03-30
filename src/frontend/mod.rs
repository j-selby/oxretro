//! The frontend recieves events from the core, as well as manages the state of
//! various platform-specific modules (video/audio/input).

pub mod state;
pub mod protocol;

pub use self::protocol::run;
