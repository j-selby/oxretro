/// Maintains the current, global state.

use std::path::Path;
use std::fs::create_dir;
use std::fs::canonicalize;
use std::ffi::CString;

use graphics::Renderer;
use audio::AudioBackend;
use retro_types::{RetroSystemInfo, RetroVariable};

// Static callbacks
pub struct FrontendState {
    pub renderer : Option<Box<Renderer>>,
    pub audio : Option<Box<AudioBackend>>,
    pub info : Option<RetroSystemInfo>,

    pub variables : Vec<RetroVariable>,
    pub variables_dirty : bool,

    // Extract these to a FII structure
    pub save_path : CString,
    pub system_path : CString
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

    /// Builds a new frontend state.
    pub fn new(renderer : Option<Box<Renderer>>,
               audio : Option<Box<AudioBackend>>,
               info : Option<RetroSystemInfo>) -> FrontendState {
        let saves_dir = Path::new("saves");
        if !saves_dir.exists() {
            create_dir(&saves_dir).unwrap();
        }

        let saves_dir = canonicalize(saves_dir).unwrap().to_str().unwrap().to_owned();

        let systems_dir = Path::new("system");
        if !systems_dir.exists() {
            create_dir(&systems_dir).unwrap();
        }

        let systems_dir = canonicalize(systems_dir).unwrap().to_str().unwrap().to_owned();

        println!("Save path: {}", saves_dir);

        FrontendState {
            renderer,
            audio,
            info,
            variables : Vec::new(),
            variables_dirty : true,

            save_path : CString::new(saves_dir).unwrap(),
            system_path : CString::new(systems_dir).unwrap()
        }
    }
}
