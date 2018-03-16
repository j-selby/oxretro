/// The various types in the LibRetro API.

use std::ffi::CStr;
use std::ffi::CString;

use std::str::Utf8Error;

use std::os::raw::*;

// Core functions
// retro_init()
pub type RetroInitFn = unsafe extern fn() -> ();

// retro_deinit()
pub type RetroDeinitFn = unsafe extern fn() -> ();

// unsigned retro_api_version()
pub type RetroApiVersionFn = unsafe extern fn() -> c_uint;

// retro_run()
pub type RetroRunFn = unsafe extern fn() -> ();

// retro_reset()
pub type RetroResetFn = unsafe extern fn() -> ();

// bool retro_get_system_info(const struct retro_system_info*)
pub type RetroGetSystemInfoFn = unsafe extern fn(*mut RawRetroSystemInfo) -> bool;

// retro_set_environment(retro_environment_t)
pub type RetroSetEnvironmentFn = unsafe extern fn(unsafe extern fn(c_uint, *const c_void) -> bool) -> ();

// retro_set_video_refresh(retro_environment_t)
pub type RetroSetVideoRefreshFn = unsafe extern fn(unsafe extern fn(*const c_void, c_uint, c_uint, usize)) -> ();

// retro_set_video_refresh(retro_environment_t)
pub type RetroSetAudioSampleFn = unsafe extern fn(unsafe extern fn(i16, i16)) -> ();

// retro_set_video_refresh(retro_environment_t)
pub type RetroSetAudioSampleBatchFn = unsafe extern fn(unsafe extern fn(*const i16, usize)) -> ();

// retro_set_video_refresh(retro_environment_t)
pub type RetroSetInputPollFn = unsafe extern fn(unsafe extern fn()) -> ();

// retro_set_video_refresh(retro_environment_t)
pub type RetroSetInputStateFn = unsafe extern fn(unsafe extern fn(c_uint, c_uint, c_uint, c_uint) -> i16) -> ();

// bool retro_load_game(const struct retro_game_info*)
pub type RetroLoadGameFn = unsafe extern fn(*const RawRetroGameInfo) -> bool;

// void retro_get_system_av_info(struct retro_system_av_info*)
pub type RetroGetSystemAvInfoFn = unsafe extern fn(*const RetroAvInfo) -> ();

/// Converts a C char array to a owned Rust String. Helper to other functions in here.
fn char_pointer_to_owned(string : *const c_char) -> Result<String, Utf8Error> {
    Ok(
        unsafe {
            CStr::from_ptr(string)
        }.to_str()?.to_owned()
    )
}

/// Raw, C-compatible version of RetroSystemInfo for FFI.
#[repr(C)]
pub struct RawRetroSystemInfo {
    library_name : *mut c_char,
    library_version : *mut c_char,
    valid_extensions : *mut c_char,
    need_fullpath : bool,
    block_extract : bool
}

impl RawRetroSystemInfo {
    /// Converts this structures contents into a owned, safe type.
    pub fn into_owned(self) -> Result<RetroSystemInfo, Utf8Error> {
        Ok(
            RetroSystemInfo {
                library_name : char_pointer_to_owned(self.library_name)?,
                library_version : char_pointer_to_owned(self.library_version)?,
                valid_extensions : char_pointer_to_owned(self.valid_extensions)?
                    .split("|").map(|x| x.to_owned()).collect(),
                need_fullpath : self.need_fullpath,
                block_extract : self.block_extract
            }
        )
    }

    /// Creates a new RawRetroSystemInfo, ready for FFI.
    pub fn new() -> Self {
        RawRetroSystemInfo {
            library_name: 0 as _,
            library_version: 0 as _,
            valid_extensions: 0 as _,
            need_fullpath: false,
            block_extract: false
        }
    }
}

/// Describes the metadata for a particular core.
#[derive(Debug)]
pub struct RetroSystemInfo {
    pub library_name : String,
    pub library_version : String,
    pub valid_extensions : Vec<String>,
    pub need_fullpath : bool,
    pub block_extract : bool
}

/// FFI version of RetroGameInfo.
#[repr(C)]
pub struct RawRetroGameInfo {
    path : *const c_char,
    data : *const c_void,
    size : usize,
    meta : *const c_char
}

/// Describes what game is meant to be loaded.
pub struct RetroGameInfo {
    pub path : Option<CString>,
    pub data : Option<Vec<u8>>,
    pub size : usize,
    pub meta : Option<CString>
}

impl RetroGameInfo {
    /// Converts this type to a FFI compatible one.
    pub fn as_raw<'a>(&'a self) -> RawRetroGameInfo {
        RawRetroGameInfo {
            path: match self.path {
                Some(ref v) => v.as_ptr() as _,
                _ => 0 as _
            },
            data: match self.data {
                Some(ref v) => v.as_ptr() as _,
                _ => 0 as _
            },
            size: self.size,
            meta: match self.meta {
                Some(ref v) => v.as_ptr() as _,
                _ => 0 as _
            }
        }
    }

    /// Creates a new RetroGameInfo.
    pub fn new(path : Option<&str>, data : Option<Vec<u8>>, size : usize,
           meta : Option<&str>) -> RetroGameInfo {
        RetroGameInfo {
            path: match path {
                Some(v) => Some(CString::new(v).unwrap()),
                _ => None
            },
            data: match data {
                Some(v) => Some(v),
                _ => None
            },
            size,
            meta: match meta {
                Some(v) => Some(CString::new(v).unwrap()),
                _ => None
            }
        }
    }
}

/// Describes the dimensions of a core's requested framebuffer.
#[repr(C)]
pub struct RetroGameGeometry {
    pub base_width : u32,
    pub base_height : u32,
    pub max_width : u32,
    pub max_height : u32,
    pub aspect_ratio : f32
}

/// Describes the timings of a core.
#[repr(C)]
pub struct RetroSystemTiming {
    pub fps : f64,
    pub sample_rate : f64
}

/// Describes the A/V requirements for a Core.
#[repr(C)]
pub struct RetroAvInfo {
    pub geometry : RetroGameGeometry,
    pub timing : RetroSystemTiming
}

impl RetroAvInfo {
    /// Creates a new FFI compatible RetroAvInfo.
    pub fn new() -> Self {
        RetroAvInfo {
            geometry: RetroGameGeometry {
                base_width: 0,
                base_height: 0,
                max_width: 0,
                max_height: 0,
                aspect_ratio: 0.0,
            },
            timing: RetroSystemTiming { fps: 0.0, sample_rate: 0.0 },
        }
    }
}

/// Describes various formats for framebuffers that can be sent across the API.
#[derive(Debug, Copy, Clone)]
pub enum RetroPixelFormat {
    /* 0RGB1555, native endian.
     * 0 bit must be set to 0.
     * This pixel format is default for compatibility concerns only.
     * If a 15/16-bit pixel format is desired, consider using RGB565. */
    Format0RGB1555 = 0,

    /* XRGB8888, native endian.
     * X bits are ignored. */
    FormatXRGB8888 = 1,

    /* RGB565, native endian.
     * This pixel format is the recommended format to use if a 15/16-bit
     * format is desired as it is the pixel format that is typically
     * available on a wide range of low-power devices.
     *
     * It is also natively supported in APIs like OpenGL ES. */
    FormatRGB565 = 2
}

impl RetroPixelFormat {
    /// Returns the size of a pixel in bytes.
    pub fn get_pixel_size(&self) -> usize {
        match self {
            &RetroPixelFormat::Format0RGB1555 => 2,
            &RetroPixelFormat::FormatXRGB8888 => 4,
            &RetroPixelFormat::FormatRGB565 => 2
        }
    }

    /// Converts from this pixel format to ARGB8888.
    pub fn convert(&self, original_data : &[u8], width : usize, height : usize) -> Vec<u8> {
        let mut result: Vec<u8> = Vec::with_capacity(width * height * 4);

        let pixel_size = self.get_pixel_size();

        match self {
            &RetroPixelFormat::Format0RGB1555 => {
                for y in 0 .. height {
                    for x in 0 .. width {
                        let pos = (y * width + x) * pixel_size;
                        let data = ((original_data[pos + 1] as u16) << 8) |
                            (original_data[pos] as u16);

                        let r = (data >> 10) & 0b11111;
                        let g = (data >> 5) & 0b11111;
                        let b = (data >> 0) & 0b11111;

                        let scaled_r = ((r as f64) / 32.0 * 256.0) as u8;
                        let scaled_g = ((g as f64) / 32.0 * 256.0) as u8;
                        let scaled_b = ((b as f64) / 32.0 * 256.0) as u8;

                        result.push(scaled_r);
                        result.push(scaled_g);
                        result.push(scaled_b);
                        result.push(255);
                    }
                }
            },
            &RetroPixelFormat::FormatXRGB8888 => {
                for y in 0 .. height {
                    for x in 0 .. width {
                        let pos = y * width * pixel_size + x * pixel_size;

                        result.push(original_data[pos]);
                        result.push(original_data[pos + 1]);
                        result.push(original_data[pos + 2]);
                        result.push(255);
                    }
                }
            },
            &RetroPixelFormat::FormatRGB565 => {
                for y in 0 .. height {
                    for x in 0 .. width {
                        let pos = (y * width + x) * pixel_size;
                        let data = ((original_data[pos + 1] as u16) << 8) |
                            (original_data[pos] as u16);

                        let r = (data >> 11) & 0b11111;
                        let g = (data >> 5) & 0b111111;
                        let b = (data >> 0) & 0b11111;

                        let scaled_r = ((r * 527 + 23) >> 6) as u8;
                        let scaled_g = ((g * 259 + 33) >> 6) as u8;
                        let scaled_b = ((b * 527 + 23) >> 6) as u8;

                        result.push(scaled_r);
                        result.push(scaled_g);
                        result.push(scaled_b);
                        result.push(255);
                    }
                }
            }
        }

        result
    }

    /// Converts from a raw integer format, as used by the LibRetro API.
    pub fn from(format : u32) -> Option<RetroPixelFormat> {
        Some(
            match format {
                0 => RetroPixelFormat::Format0RGB1555,
                1 => RetroPixelFormat::FormatXRGB8888,
                2 => RetroPixelFormat::FormatRGB565,
                _ => return None
            }
        )
    }
}