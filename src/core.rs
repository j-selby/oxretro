/// Represents an external LibRetroCore.

use lib;

use std::io;
use std::fs::File;
use std::str::Utf8Error;
use std::path::Path;
use std::io::Read;

use retro_types::*;
use callbacks::*;

// Core interface
pub struct LibRetroCore {
    library : lib::Library
}

// Error handling
#[derive(Debug)]
pub enum CoreError {
    BadLibrary(io::Error),
    BadEncoding(Utf8Error),
    FileLoadError(io::Error)
}

fn translate_lib_result<T>(result : Result<T, io::Error>) -> Result<T, CoreError> {
    match result {
        Ok(v) => Ok(v),
        Err(v) => Err(CoreError::BadLibrary(v))
    }
}

fn translate_encoding_result<T>(result : Result<T, Utf8Error>) -> Result<T, CoreError> {
    match result {
        Ok(v) => Ok(v),
        Err(v) => Err(CoreError::BadEncoding(v))
    }
}

/// TODO: Make this abstract
impl LibRetroCore {
    pub fn configure_callbacks(&self) -> Result<(), CoreError> {
        unsafe {
            let environment: lib::Symbol<RetroSetEnvironmentFn> =
                translate_lib_result(self.library.get(b"retro_set_environment"))?;
            let video_refresh: lib::Symbol<RetroSetVideoRefreshFn> =
                translate_lib_result(self.library.get(b"retro_set_video_refresh"))?;
            let audio_sample: lib::Symbol<RetroSetAudioSampleFn> =
                translate_lib_result(self.library.get(b"retro_set_audio_sample"))?;
            let audio_batch: lib::Symbol<RetroSetAudioSampleBatchFn> =
                translate_lib_result(self.library.get(b"retro_set_audio_sample_batch"))?;
            let input_poll: lib::Symbol<RetroSetInputPollFn> =
                translate_lib_result(self.library.get(b"retro_set_input_poll"))?;
            let input_state: lib::Symbol<RetroSetInputStateFn> =
                translate_lib_result(self.library.get(b"retro_set_input_state"))?;

            environment(environment_callback);
            video_refresh(video_refresh_callback);
            audio_sample(audio_sample_callback);
            audio_batch(audio_sample_batch_callback);
            input_poll(input_poll_callback);
            input_state(input_state_callback);
        }

        Ok(())
    }

    pub fn init(&self) -> Result<(), CoreError> {
        unsafe {
            let func: lib::Symbol<RetroInitFn> =
                translate_lib_result(self.library.get(b"retro_init"))?;

            func();
        }

        Ok(())
    }

    pub fn load_game(&self, path : Option<&Path>) -> Result<bool, CoreError> {
        let info = self.get_system_info()?;

        let meta = match path {
            Some(v) => {
                Some(
                    if info.need_fullpath {
                        RetroGameInfo::new(v.to_str(), None,
                                           translate_lib_result(v.metadata())?.len() as _, Some(""))
                    } else {
                        let length : usize;
                        let data = {
                            let mut file = translate_lib_result(File::open(v))?;
                            let mut buf = Vec::new();
                            length = translate_lib_result(file.read_to_end(&mut buf))?;
                            buf
                        };

                        RetroGameInfo::new(v.to_str(), Some(data),length, Some(""))
                    }
                )
            }
            _ => None
        };

        unsafe {
            let func: lib::Symbol<RetroLoadGameFn> =
                translate_lib_result(self.library.get(b"retro_load_game"))?;

            match meta {
                Some(v) => Ok(func((&v.as_raw()) as *const RawRetroGameInfo)),
                None => Ok(func(0 as *const RawRetroGameInfo))
            }

        }
    }

    pub fn deinit(&self) -> Result<(), CoreError> {
        unsafe {
            let func: lib::Symbol<RetroDeinitFn> =
                translate_lib_result(self.library.get(b"retro_deinit"))?;

            func();
        }

        Ok(())
    }

    pub fn get_system_info(&self) -> Result<RetroSystemInfo, CoreError> {
        let mut core_info = RawRetroSystemInfo::new();

        unsafe {
            let func: lib::Symbol<RetroGetSystemInfoFn> =
                translate_lib_result(self.library.get(b"retro_get_system_info"))?;

            func(&mut core_info);
        }

        Ok(translate_encoding_result(core_info.into_owned())?)
    }

    pub fn get_av_info(&self) -> Result<RetroAvInfo, CoreError> {
        let mut core_info = RetroAvInfo::new();

        unsafe {
            let func: lib::Symbol<RetroGetSystemAvInfoFn> =
                translate_lib_result(self.library.get(b"retro_get_system_av_info"))?;

            func(&mut core_info);
        }

        Ok(core_info)
    }

    pub fn run(&self) -> Result<(), CoreError> {
        unsafe {
            let func: lib::Symbol<RetroRunFn> =
                translate_lib_result(self.library.get(b"retro_run"))?;

            func();
        }

        Ok(())
    }

    pub fn from_library(library : lib::Library) -> LibRetroCore {
        LibRetroCore {
            library
        }
    }
}
