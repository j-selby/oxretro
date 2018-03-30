/// Helper FFI functions

use std::ffi::CStr;
use std::os::raw::*;
use std::str::Utf8Error;

/// Converts a C char array to a owned Rust String. Helper to other functions in here.
pub fn char_pointer_to_owned(string: *const c_char) -> Result<String, Utf8Error> {
    Ok(unsafe { CStr::from_ptr(string) }.to_str()?.to_owned())
}
