#![feature(box_leak)]

extern crate libloading as lib;

mod audio;
mod input;
mod graphics;

mod state;
mod retro_types;
mod core;
mod callbacks;

use core::LibRetroCore;

use state::FrontendState;

use retro_types::RetroPixelFormat;

use std::path::Path;

use std::{thread, time};

fn main() {
    println!("Loading library...");
    let library = lib::Library::new("core.dll").unwrap();

    println!("Configuring environment...");
    let core = LibRetroCore::from_library(library);

    println!("Core info:");
    let info = core.get_system_info().unwrap();
    println!("{:?}", info);

    let mut frontend = FrontendState::new(None, None, info,
                                          RetroPixelFormat::Format0RGB1555);

    unsafe {
        frontend.make_current();
    }

    core.configure_callbacks().unwrap();

    println!("Core init:");
    core.init().unwrap();


    println!("Load:");
    println!("{:?}", core.load_game(Path::new("rom.gba")));

    println!("Building context...");
    let renderer = graphics::build(false, false).unwrap();

    println!("Av:");
    let av_info = core.get_av_info().unwrap();

    println!("Endgame:");
    frontend.renderer = Some(renderer);

    let audio = audio::build(av_info.timing.sample_rate as u32).unwrap();

    frontend.audio = Some(audio);

    println!("Loop:");
    let max_frame = time::Duration::from_millis(16);

    while frontend.is_alive() {
        let start_loop = time::Instant::now();

        core.run().unwrap();

        let elapsed = start_loop.elapsed();
        if elapsed < max_frame {
            let sleep_time = max_frame - elapsed;

            thread::sleep(sleep_time);
        }
    }

    println!("Core deinit:");
    core.deinit().unwrap();

    println!("All done!");
}
