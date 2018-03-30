//! The backend talks with the LibRetro core, and relays its commands through the IPC
//! mechanisms available to the frontend.

extern crate libloading as lib;

pub mod core;
pub mod callbacks;
pub mod protocol;
pub mod state;

pub use self::protocol::run;
