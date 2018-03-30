#![feature(vec_remove_item)]
#![feature(duration_from_micros)]

extern crate bincode;
extern crate serde;
#[macro_use]
extern crate serde_derive;

extern crate byteorder;

#[macro_use]
extern crate clap;

mod audio;
mod input;
mod graphics;
mod backend;
mod frontend;

mod retro_types;
mod ffi;
mod core_protocol;

use clap::{App, Arg};

fn main() {
    let matches = App::new("OxRetro")
        .version(crate_version!())
        .author("Selby <jselby@jselby.net>")
        .about("A multi-process LibRetro implementation")
        .arg(
            Arg::with_name("type")
                .long("type")
                .default_value("frontend")
                .help("The kind of process that should be started")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("core")
                .long("core")
                .help("The core to load")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("address")
                .long("address")
                .help("address:port of the frontend to connect to, or to bind to")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("rom")
                .long("rom")
                .help("[Frontend only] The rom to load")
                .takes_value(true),
        )
        .arg(
            Arg::with_name("no-backend")
                .long("no-backend")
                .help("[Frontend only] Starts a frontend without an associated backend"),
        )
        .get_matches();

    let process_type = matches.value_of("type").unwrap();
    match &process_type {
        &"frontend" => {
            let core = matches.value_of("core").map(|v| v.to_owned());
            let address = matches.value_of("address").map(|v| v.to_owned());
            let rom = matches.value_of("rom").unwrap().to_owned();
            let spawn_core = matches.is_present("no-backend");

            frontend::run(core, rom, address, spawn_core);
        }
        &"backend" => {
            let address = matches.value_of("address").unwrap().to_owned();
            let core = matches.value_of("core").unwrap().to_owned();

            backend::run(core, address);
        }
        _ => panic!("Unknown type: {}", process_type),
    }
}
