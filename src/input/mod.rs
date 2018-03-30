/// Abstracts input frontends.

#[cfg(feature = "input_gilrs")]
pub mod gilrs;

/// Keys that can be pressed on a controller/"RetroPad".
pub enum InputKey {
    A,
    B,
    X,
    Y,
    Select,
    Start,
    Up,
    Down,
    Left,
    Right,
    L,
    R,
    L2,
    R2,
    L3,
    R3,
}

#[derive(Debug)]
pub struct InputBackendInfo {
    name: &'static str,
}

pub trait InputBackend {
    fn poll_events(&mut self);

    fn is_key_down(&self, key: &InputKey) -> bool;
}

static AVAILABLE_BACKENDS: &'static [(&'static InputBackendInfo, fn() -> Box<InputBackend>)] = &[
    #[cfg(feature = "input_gilrs")]
    (&gilrs::INFO, gilrs::build),
];

/// Builds a new renderer with the specified properties.
pub fn build() -> Option<Box<InputBackend>> {
    for &(ref info, ref function) in AVAILABLE_BACKENDS {
        println!("Attempting to load input core: {:?}", info);

        return Some(function());
    }

    return None;
}
