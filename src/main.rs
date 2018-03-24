#![feature(vec_remove_item)]

extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate bincode;

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

use clap::{Arg, App};

fn main() {
    let matches = App::new("OxRetro")
        .version(crate_version!())
        .author("Selby <jselby@jselby.net>")
        .about("A multi-process LibRetro implementation")
        .arg(Arg::with_name("type")
            .long("type")
            .default_value("frontend")
            .help("Internal use only")
            .takes_value(true))
        .arg(Arg::with_name("port")
            .long("port")
            .help("Internal use only")
            .takes_value(true))
        .arg(Arg::with_name("core")
            .long("core")
            .help("The core to load")
            .takes_value(true))
        .arg(Arg::with_name("rom")
            .long("rom")
            .help("The rom to load")
            .takes_value(true))
        .get_matches();

    match matches.value_of("type").unwrap() {
        "frontend" => {
            let core = matches.value_of("core").unwrap().to_owned();
            let rom = matches.value_of("rom").unwrap().to_owned();

            frontend::run(core, rom);
        },
        "backend" => {
            let port = matches.value_of("port").unwrap().parse::<u16>().unwrap();
            let core = matches.value_of("core").unwrap().to_owned();

            backend::run(core, port);
        },
        _ => {
            panic!("Unknown type!")
        }
    }
}
