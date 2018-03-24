use std::mem::transmute;

use std::ffi::CString;

use std::fs::canonicalize;
use std::fs::create_dir;

use std::path::Path;

use retro_types::RetroPixelFormat;

// Static callbacks
pub struct BackendState {
    pub format : RetroPixelFormat,

    // Extract these to a FII structure
    pub save_path : CString,
    pub system_path : CString,

    is_global : bool
}

impl BackendState {
    /// Makes this core the current, global instance.
    pub unsafe fn make_current(&mut self) {
        if BACKEND.is_some() {
            panic!("Multiple backends active at once!");
        }

        BACKEND = Some(self as *mut BackendState);

        self.is_global = true;
    }

    /// Removes this core from the global state, if it has already been set as global.
    pub unsafe fn done_current(&mut self) {
        if self.is_global {
            BACKEND.take();
            self.is_global = false;
        }
    }

    /// Builds a new frontend state.
    pub fn new(format : RetroPixelFormat) -> BackendState {
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

        BackendState {
            format,

            save_path : CString::new(saves_dir).unwrap(),
            system_path : CString::new(systems_dir).unwrap(),

            is_global: false
        }
    }
}

impl Drop for BackendState {
    fn drop(&mut self) {
        unsafe {
            self.done_current();
        }
    }
}

/// Reference to the current frontend. Necessary for
static mut BACKEND : Option<*mut BackendState> = None;

/// Returns the current frontend, or panics if one is not available.
pub fn get_current_backend() -> &'static mut BackendState {
    unsafe {
        transmute(BACKEND.unwrap())
    }
}
