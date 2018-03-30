/// Maintains the current, global state.

use graphics::Renderer;
use audio::AudioBackend;
use retro_types::{RetroSystemInfo, RetroVariable};

// Static callbacks
pub struct FrontendState {
    pub renderer: Option<Box<Renderer>>,
    pub audio: Option<Box<AudioBackend>>,
    pub info: Option<RetroSystemInfo>,

    pub variables: Vec<RetroVariable>,
    pub variables_dirty: bool,
}

impl FrontendState {
    /// Polls the input backend for available input.
    pub fn poll_input(&mut self) {
        match &mut self.renderer {
            &mut Some(ref mut v) => v.poll_events(),
            &mut None => panic!("No renderer when input callback was called!"),
        };
    }

    /// Checks to see if all the components are alive.
    pub fn is_alive(&self) -> bool {
        match &self.renderer {
            &Some(ref v) => v.is_alive(),
            &None => {
                panic!("No renderer when input callback was called!");
            }
        }
    }

    /// Builds a new frontend state.
    pub fn new(
        renderer: Option<Box<Renderer>>,
        audio: Option<Box<AudioBackend>>,
        info: Option<RetroSystemInfo>,
    ) -> FrontendState {
        FrontendState {
            renderer,
            audio,
            info,
            variables: Vec::new(),
            variables_dirty: true,
        }
    }
}
