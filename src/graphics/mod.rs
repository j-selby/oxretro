/// The renderer presents content to the user.
///
/// This is equal to a video driver in RetroArch.

#[cfg(feature = "graphics_opengl")]
pub mod gl;

use input::InputKey;

#[derive(Debug)]
pub struct RendererInfo {
    name : &'static str,
    provides_opengl : bool,
    provides_vulkan : bool
}

pub trait Renderer {
    fn submit_frame(&mut self, frame : &[u8], width : usize, height : usize);

    // TODO: This shouldn't be here
    fn poll_events(&mut self);

    fn is_alive(&self) -> bool;

    // TODO: This shouldn't be here
    fn is_key_down(&self, key : &InputKey) -> bool;
}

static AVAILABLE_RENDERERS: &'static [(&'static RendererInfo, fn() -> Box<Renderer>)] = &[
        #[cfg(feature = "graphics_opengl")]
        (&gl::INFO, gl::build)
];

/// Builds a new renderer with the specified properties.
pub fn build(needs_opengl : bool, needs_vulkan : bool) -> Option<Box<Renderer>> {
    for &(ref info, ref function) in AVAILABLE_RENDERERS {
        if needs_opengl && !info.provides_opengl {
            continue;
        }

        if needs_vulkan && !info.provides_vulkan {
            continue;
        }

        println!("Attempting to load video core: {:?}", info);

        return Some(function());
    }

    return None;
}
