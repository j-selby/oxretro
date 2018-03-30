//! Handlers for the C callbacks from the LibRetro core itself.
//! These are generally unsafe, and have to use a global callback in order to talk to the main
//! application.

use retro_types::RetroPixelFormat;
use retro_types::RetroEnvironment;
use retro_types::RawRetroVariable;

use backend::state::get_current_backend;
use backend::protocol::send_message;

use std::os::raw::*;
use std::mem::transmute;
use std::slice::from_raw_parts;

use core_protocol::ProtocolMessageType;
use core_protocol::VideoRefreshType;

/// Misc. environment calls.
///
pub unsafe extern "C" fn environment_callback(cmd: c_uint, data: *const c_void) -> bool {
    // Mask out flags - they are for API defintions mainly, and we either
    // support the specfic feature or not.
    let cmd = cmd & 0xFFFF;

    let safe_command = RetroEnvironment::from_command_id(cmd);

    let safe_command = match safe_command {
        Some(v) => v,
        None => {
            // Unsupported command
            println!("Unknown environmental command: {}", cmd);
            return false;
        }
    };

    match safe_command {
        RetroEnvironment::SetPixelFormat => {
            // This call remains local
            let raw_pixel_format = *(data as *const u32);

            let actual_pixel_format = RetroPixelFormat::from(raw_pixel_format);

            match actual_pixel_format {
                Some(value) => {
                    let backend = get_current_backend();
                    backend.format = value;
                    true
                }
                _ => false,
            }
        }
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

            send_message(ProtocolMessageType::SetVariables(strings));
            true
        }
        RetroEnvironment::GetVariable => {
            let variable = &mut *(data as *mut RawRetroVariable);
            let key = variable.get_key().unwrap();

            let mut found = false;

            let result = send_message(ProtocolMessageType::GetVariable(key))
                .unwrap()
                .unwrap();

            let data = match result {
                ProtocolMessageType::GetVariableResponse(result) => result,
                _ => panic!("Bad response to message!"),
            };

            /*for search_variable in &frontend.variables {
                if key == search_variable.key {
                    // This is UNSAFE, but frontend does exist until core_deinit, and
                    // the core shouldn't be able to refer to it beyond there.
                    variable.value = search_variable.get_selected();
                    found = true;
                    break;
                }
            }*/

            // TODO: How to do this safely?
            /*match data {
                Some(v) => {
                    variable.value = v.get;
                }
            }*/

            false

            //found
        }
        RetroEnvironment::GetVariableUpdate => {
            // TODO: Check frontend for this one
            //let frontend = get_current_frontend();
            //*(data as *mut bool) = frontend.variables_dirty;

            true
        }
        RetroEnvironment::GetSaveDirectory => {
            let frontend = get_current_backend();
            *(data as *mut *const c_char) = frontend.save_path.as_ptr() as *const _;
            true
        }
        RetroEnvironment::GetSystemDirectory => {
            let frontend = get_current_backend();
            *(data as *mut *const c_char) = frontend.system_path.as_ptr() as *const _;
            true
        }
        _ => {
            println!("Unsupported environmental command: {:?}", safe_command);
            false
        }
    }
}

pub unsafe extern "C" fn video_refresh_callback(
    data: *const c_void,
    width: c_uint,
    height: c_uint,
    pitch: usize,
) {
    let width = width as usize;
    let height = height as usize;

    if data != 0 as *const _ {
        // Software refresh
        let format = get_current_backend().format;
        let pixel_size = format.get_pixel_size();
        let mut padless_data: Vec<u8> = Vec::with_capacity(width * height * pixel_size);

        // Copy the data (which can have a pitch of > 0) into our own safe array
        if width > 0 && height > 0 && pitch > 0 {
            assert!(padless_data.len() <= pitch * height);

            // c_void isn't a particularly useful type - we have to transmute
            let raw_data: &[u8] = transmute(from_raw_parts(data, pitch * height));

            for y in 0..height {
                padless_data
                    .extend_from_slice(&raw_data[y * pitch..(y * pitch + width * pixel_size)]);
            }
        }

        assert_eq!(padless_data.len(), width * height * pixel_size);

        let formatted_data = format.convert(&padless_data, width, height);

        send_message(ProtocolMessageType::VideoRefresh(
            VideoRefreshType::Software {
                framebuffer: formatted_data,
                width: width as u64,
                height: height as u64,
            },
        ));
    } else {
        // Hardware callback
        send_message(ProtocolMessageType::VideoRefresh(
            VideoRefreshType::Hardware,
        ));
    }
}

pub unsafe extern "C" fn audio_sample_callback(left: i16, right: i16) {
    //println!("Single audio callback - redirecting...");
    let data = [left, right];
    audio_sample_batch_callback(data.as_ptr(), 1)
}

pub unsafe extern "C" fn audio_sample_batch_callback(data: *const i16, frames: usize) {
    let data = from_raw_parts(data, frames * 2);

    send_message(ProtocolMessageType::AudioSample(data.to_owned()));
}

pub unsafe extern "C" fn input_poll_callback() {
    send_message(ProtocolMessageType::PollInput);
}

pub unsafe extern "C" fn input_state_callback(
    port: c_uint,
    device: c_uint,
    index: c_uint,
    id: c_uint,
) -> i16 {
    // TODO: Make backends abstract
    /*let frontend = get_current_frontend();
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
    };*/

    match send_message(ProtocolMessageType::InputState {
        port,
        device,
        index,
        id,
    }).unwrap()
        .unwrap()
    {
        ProtocolMessageType::InputResponse(v) => v,
        _ => panic!("Unexpected input response!"),
    }
}
