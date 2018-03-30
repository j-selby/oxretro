/// The renderer presents audio to the user.
///
/// This is equal to a audio driver in RetroArch.

#[cfg(feature = "audio_cpal")]
pub mod cpal;

#[derive(Debug)]
pub struct AudioBackendInfo {
    name : &'static str
}

pub trait AudioBackend {
    fn submit_frame(&mut self, frames : &[i16]);

    fn is_done(&self) -> bool;

    fn get_done_callback(&self) -> Box<Fn() -> bool + Send>;
}

static AVAILABLE_AUDIO_BACKENDS: &'static [(&'static AudioBackendInfo, fn(u32)
    -> Box<AudioBackend>)] = &[
    #[cfg(feature = "audio_cpal")]
    (&cpal::INFO, cpal::build)
];

/// Builds a new renderer with the specified properties.
pub fn build(sample_rate : u32) -> Option<Box<AudioBackend>> {
    for &(ref info, ref function) in AVAILABLE_AUDIO_BACKENDS {
        println!("Attempting to load audio core: {:?}", info);
        return Some(function(sample_rate));
    }

    return None;
}
