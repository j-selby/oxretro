/// Handles Frontend<->Backend communication

use core_protocol::ProtocolAdapter;
use core_protocol::ProtocolMessageType;

use frontend::state::FrontendState;

use std::env::current_exe;

use std::net::TcpListener;

use std::process::Command;
use std::process::Stdio;

use std::thread;
use graphics;
use audio;
use core_protocol::VideoRefreshType;
use input::InputKey;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::Sender;
use std::time::Duration;

/// Starts listening for messages over a socket. Binds to the port as a server.
pub fn run(core : Option<String>, rom : String, address : Option<String>, dont_spawn_core : bool) {
    // Bind to our target port
    let server = match address {
        Some(v) => TcpListener::bind(v).unwrap(),
        None => TcpListener::bind("127.0.0.1:0").unwrap()
    };

    let port = server.local_addr().unwrap().port();

    // Start up a client
    if !dont_spawn_core {
        let exe_path = current_exe().unwrap();
        let _process = Command::new(exe_path)
            .arg("--type").arg("backend")
            .arg("--address").arg(&format!("127.0.0.1:{}", port))
            .arg("--core").arg(&core.unwrap())
            .stdin(Stdio::null())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .spawn()
            .expect("Unable to launch core process");
    }

    // Wait for this client to come online
    println!("Waiting for client...");
    let (stream, _) = server.accept().unwrap();
    println!("Client online!");

    let mut frontend = FrontendState::new(None, None, None);

    let stdin = Box::new(stream.try_clone().unwrap());
    let stdout = Box::new(stream.try_clone().unwrap());
    // TODO: Handle events
    let (protocol, events) = ProtocolAdapter::new("frontend".to_owned(),
                                                  stdin, stdout);

    // Request system info
    let data =
        match protocol.send(ProtocolMessageType::SystemInfo).unwrap().unwrap() {
            ProtocolMessageType::SystemInfoResponse(info) => info,
            _ => panic!("Bad response to system info!")
        };
    println!("Loaded core: {:?}", data.library_name);
    frontend.info = Some(data);

    let av_info = match protocol.send(ProtocolMessageType::AVInfo)
        .unwrap().unwrap() {
        ProtocolMessageType::AVInfoResponse(info) => info,
        _ => panic!("Unknown A/V info")
    };

    protocol.send(ProtocolMessageType::Init);
    protocol.send(ProtocolMessageType::Load(rom));

    // Finish up our frontend
    let mut renderer = graphics::build(false, false).unwrap();

    match &frontend.info {
        &Some(ref v) => renderer.set_title(format!("OxRetro - {} ({})", v.library_name,
                                       v.library_version)),
        _ => panic!("Missing frontend info?")
    }

    frontend.renderer = Some(renderer);

    let audio = audio::build(av_info.timing.sample_rate as u32).unwrap();
    let audio_size_callback = audio.get_done_callback();
    frontend.audio = Some(audio);

    // Signaling for the start of a frame. true if frames should continue to be sent
    let (frame_tx, frame_rx): (Sender<bool>,
                               Receiver<bool>) = mpsc::channel();

    // Create a thread for managing events
    thread::Builder::new().name("frontend-events".to_owned()).spawn(move || {
        loop {
            protocol.send(ProtocolMessageType::Run);

            // TODO: busy loop
            while !audio_size_callback() {
                thread::sleep(Duration::from_millis(1));
            }

            if !frame_rx.recv().unwrap() {
                protocol.send(ProtocolMessageType::Unload);
                break;
            }
        }
    }).unwrap();

    // Start up our main loop - we no longer need to talk to the frontend
    loop {
        let (event, callback) = match events.poll() {
            Some(v) => v,
            None => {
                println!("Frontend was disconnected from client.");
                break
            }
        };

        match event {
            ProtocolMessageType::GetVariable(name) => callback(ProtocolMessageType::GetVariableResponse(None)),
            ProtocolMessageType::PollInput => frontend.poll_input(),
            ProtocolMessageType::InputState { id, .. } => {
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
                    _ => panic!("Unknown input ID: {}", id)
                };

                let result : i16;
                match &mut frontend.renderer {
                    &mut Some(ref mut v) => {
                        if v.is_key_down(&key) {
                            result = 1;
                        } else {
                            result = 0;
                        }
                    },
                    &mut None => panic!("No renderer available!")
                }

                callback(ProtocolMessageType::InputResponse(result));
            },
            ProtocolMessageType::VideoRefresh(refresh) => {
                match &mut frontend.renderer {
                    &mut Some(ref mut v) => {
                        match refresh {
                            VideoRefreshType::Software { framebuffer, width, height } => {
                                v.submit_frame(&framebuffer, width as usize, height as usize);
                            },
                            VideoRefreshType::Hardware => panic!("Hardware accelerated cores not supported!")
                        }
                    },
                    &mut None => panic!("No renderer available!")
                }

                if !frontend.is_alive() {
                    break;
                }

                frame_tx.send(true).unwrap();
            },
            ProtocolMessageType::AudioSample(samples) => {
                match &mut frontend.audio {
                    &mut Some(ref mut v) => {
                        v.submit_frame(&samples);
                    },
                    &mut None => panic!("No audio core available!")
                }
            },
            _ => {
                //println!("Ignoring!")
            }
        }
    }

    frame_tx.send(false).unwrap();
}
