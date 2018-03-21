/// Handlers for the callbacks from the LibRetro core.

use retro_types::RetroPixelFormat;
use retro_types::RetroEnvironment;
use retro_types::RawRetroVariable;

use ffi::char_pointer_to_owned;

use state::get_current_frontend;

use input::InputKey;

use std::os::raw::*;
use std::mem::transmute;
use std::slice::from_raw_parts;

pub unsafe extern "C" fn environment_callback(cmd : c_uint, data : *const c_void) -> bool {
    // Mask out flags - they are for API defintions mainly, and we either
    // support the specfic feature or not.
    let cmd = cmd & 0xFFFF;

    let safe_command = RetroEnvironment::from_command_id(cmd);

    let safe_command = match safe_command {
        Some(v) => v,
        None => {
            // Unsupported command
            println!("Unknown environmental command: {}", cmd);
            return false
        }
    };

    match safe_command {
        RetroEnvironment::SetPixelFormat => {
            let raw_pixel_format = *(data as *const u32);

            let actual_pixel_format = RetroPixelFormat::from(raw_pixel_format);

            match actual_pixel_format {
                Some(value) => {
                    let frontend = get_current_frontend();
                    frontend.format = value;
                    true
                },
                _ => false
            }
        },
        RetroEnvironment::SetVariables => {
            // Fetch all components of the structure till nullptr
            let mut strings = Vec::new();

            println!("Attempting variables:");
            let mut add = 0;
            loop {
                let inner_ptr = (data as *const RawRetroVariable).offset(add);

                let variable = &*inner_ptr;
                if variable.is_eof() {
                    break;
                }

                let variable = variable.to_owned().unwrap();

                println!("{:?}", variable);

                strings.push(variable);

                add += 1;
            }

            let frontend = get_current_frontend();
            frontend.variables = strings;
            // TODO: Set values for this from configuration file
            frontend.variables_dirty = true;

            //println!("{:?}", strings);
            true
        },
        RetroEnvironment::GetVariable => {
            let variable = &mut *(data as *mut RawRetroVariable);
            let key = variable.get_key().unwrap();

            let frontend = get_current_frontend();
            let mut found = false;

            for search_variable in &frontend.variables {
                if key == search_variable.key {
                    // This is UNSAFE, but frontend does exist until core_deinit, and
                    // the core shouldn't be able to refer to it beyond there.
                    variable.value = search_variable.get_selected();
                    found = true;
                    break;
                }
            }

            found
        },
        RetroEnvironment::GetVariableUpdate => {
            let frontend = get_current_frontend();
            *(data as *mut bool) = frontend.variables_dirty;

            true
        },
        RetroEnvironment::GetSaveDirectory => {
            let frontend = get_current_frontend();
            *(data as *mut *const c_char) = frontend.save_path.as_ptr() as *const _;
            true
        },
        RetroEnvironment::GetSystemDirectory => {
            let frontend = get_current_frontend();
            *(data as *mut *const c_char) = frontend.system_path.as_ptr() as *const _;
            true
        },
        _ => {
            println!("Unsupported environmental command: {:?}", safe_command);
            false
        }
    }
}

pub unsafe extern "C" fn video_refresh_callback(data : *const c_void, width : c_uint,
                                                height : c_uint, pitch : usize) {
    let width = width as usize;
    let height = height as usize;
    let frontend = get_current_frontend();

    if data != 0 as *const _ {
        let format = frontend.format;
        let pixel_size = format.get_pixel_size();
        let mut padless_data : Vec<u8> = Vec::with_capacity(width * height * pixel_size);

        // Copy the data (which can have a pitch of > 0) into our own safe array
        if width > 0 && height > 0 && pitch > 0 {
            assert!(padless_data.len() <= pitch * height);

            // c_void isn't a particularly useful type - we have to transmute
            let raw_data : &[u8] = transmute(from_raw_parts(data,
                                                                      pitch * height));

            for y in 0 .. height {
                padless_data.extend_from_slice(&raw_data[y * pitch .. (y * pitch + width * pixel_size)]);
            }
        }

        assert_eq!(padless_data.len(), width * height * pixel_size);

        let formatted_data = format.convert(&padless_data, width, height);

        match &mut frontend.renderer {
            &mut Some(ref mut v) => v.submit_frame(&formatted_data, width, height),
            &mut None => panic!("No renderer when draw was called!")
        };

    } else {
        println!("Null video callback!");
    }
}

pub unsafe extern "C" fn audio_sample_callback(left : i16, right : i16) {
    //println!("Single audio callback - redirecting...");
    let data = [left, right];
    audio_sample_batch_callback(data.as_ptr(), 1)
}

pub unsafe extern "C" fn audio_sample_batch_callback(data : *const i16, frames : usize) {
    let data = from_raw_parts(data, frames * 2);
    let frontend = get_current_frontend();
    match &mut frontend.audio {
        &mut Some(ref mut audio) => audio.submit_frame(data),
        _ => panic!("No audio core when audio callback was called!")
    }
    //println!("Core sent {} frames ({} parts total)", frames, data.len());
}

pub unsafe extern "C" fn input_poll_callback() {
    let frontend = get_current_frontend();
    frontend.poll_input();
}

pub unsafe extern "C" fn input_state_callback(port : c_uint, device : c_uint, index : c_uint,
                                              id : c_uint) -> i16 {
    // TODO: Make backends abstract
    let frontend = get_current_frontend();
    let key = match id {
        0 => InputKey::B,
        1 => InputKey::Y,
        2 => InputKey::Select,
        3 => InputKey::Start,
        4 => InputKey::Up,
        5 => InputKey::Down,
        6 => InputKey::Left,
        7 => InputKey::Right,
        8 => InputKey::A,
        9 => InputKey::X,
        10 => InputKey::L,
        11 => InputKey::R,
        12 => InputKey::L2,
        13 => InputKey::R2,
        14 => InputKey::L3,
        15 => InputKey::R3,
        _ => return 0
    };

    let result = match &mut frontend.renderer {
        &mut Some(ref mut v) => v.is_key_down(&key),
        &mut None => panic!("No renderer when input was called!")
    };

    if result {
        1
    } else {
        0
    }
}
