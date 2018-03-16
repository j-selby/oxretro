/// Maintains the current, global state.

use std::mem::transmute;

use graphics::Renderer;
use audio::AudioBackend;
use retro_types::{RetroSystemInfo, RetroPixelFormat};

// Static callbacks
pub struct FrontendState {
    pub renderer : Option<Box<Renderer>>,
    pub audio : Option<Box<AudioBackend>>,
    pub info : RetroSystemInfo,
    pub format : RetroPixelFormat,
    is_global : bool
}

impl FrontendState {
    /// Polls the input backend for available input.
    pub fn poll_input(&mut self) {
        match &mut self.renderer {
            &mut Some(ref mut v) => v.poll_events(),
            &mut None => panic!("No renderer when input callback was called!")
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

    /// Makes this core the current, global instance.
    pub unsafe fn make_current(&mut self) {
        if FRONTEND.is_some() {
            panic!("Multiple frontends active at once!");
        }

        FRONTEND = Some(self as *mut FrontendState);

        self.is_global = true;
    }

    /// Removes this core from the global state, if it has already been set as global.
    pub unsafe fn done_current(&mut self) {
        if self.is_global {
            FRONTEND.take();
            self.is_global = false;
        }
    }

    /// Builds a new frontend state.
    pub fn new(renderer : Option<Box<Renderer>>,
               audio : Option<Box<AudioBackend>>,
               info : RetroSystemInfo,
               format : RetroPixelFormat) -> FrontendState {
        FrontendState {
            renderer,
            audio,
            info,
            format,
            is_global: false
        }
    }
}

impl Drop for FrontendState {
    fn drop(&mut self) {
        unsafe {
            self.done_current();
        }
    }
}

/// Reference to the current frontend. Necessary for
static mut FRONTEND : Option<*mut FrontendState> = None;

/// Returns the current frontend, or panics if one is not available.
pub fn get_current_frontend() -> &'static mut FrontendState {
    unsafe {
        transmute(FRONTEND.unwrap())
    }
}
