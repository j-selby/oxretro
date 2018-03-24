/// Handles Frontend<->Backend communication

use core_protocol::ProtocolAdapter;
use core_protocol::ProtocolMessageType;

use frontend::state::FrontendState;

use std::env::current_exe;

use std::net::TcpListener;

use std::process::Command;
use std::process::Stdio;

use std::sync::Arc;
use std::sync::Mutex;
use std::thread;
use std::time;
use graphics;
use audio;
use core_protocol::VideoRefreshType;
use input::InputKey;

/// Starts listening for messages over a socket. Binds to the port as a server.
pub fn run(core : String, rom : String) {
    // Bind to our target port
    let server = TcpListener::bind("127.0.0.1:0").unwrap();

    let port = server.local_addr().unwrap().port();

    // Start up a client
    let exe_path = current_exe().unwrap();
    let process = Command::new(exe_path)
        .arg("--type").arg("backend")
        .arg("--port").arg(&format!("{}", port))
        .arg("--core").arg(&core)
        .stdin(Stdio::null())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .spawn()
        .expect("Unable to launch core process");

    // Wait for this client to come online
    println!("Waiting for client...");
    let (stream, _) = server.accept().unwrap();
    println!("Client online!");

    let frontend = FrontendState::new(None, None, None);
    // TODO: RWLock would be much better! Do for all other mutexes as well
    let frontend = Arc::new(Mutex::new(frontend));

    let stdin = Box::new(stream.try_clone().unwrap());
    let stdout = Box::new(stream.try_clone().unwrap());
    // TODO: Handle events
    let (protocol, events) = ProtocolAdapter::new(stdin,
                                                  stdout);

    // Request system info
    {
        let data =
            match protocol.send(ProtocolMessageType::SystemInfo).unwrap().unwrap() {
                ProtocolMessageType::SystemInfoResponse(info) => info,
                _ => panic!("Bad response to system info!")
            };
        frontend.lock().unwrap().info = Some(data);
    }

    println!("Got info:");
    println!("{:?}", frontend.lock().unwrap().info);

    let av_info = match protocol.send(ProtocolMessageType::AVInfo)
        .unwrap().unwrap() {
        ProtocolMessageType::AVInfoResponse(info) => info,
        _ => panic!("Unknown A/V info")
    };

    println!("AV get!");

    // Create a thread for managing events
    thread::spawn(move || {
        protocol.send(ProtocolMessageType::Init);
        protocol.send(ProtocolMessageType::Load(rom));

        let max_frame = time::Duration::from_millis(16);

        // TODO: Provide kill condition
        loop {//frontend.is_alive() {
            let start_loop = time::Instant::now();

            protocol.send(ProtocolMessageType::Run);

            let elapsed = start_loop.elapsed();
            if elapsed < max_frame {
                let sleep_time = max_frame - elapsed;

                thread::sleep(sleep_time);
            }
        }
    });

    // Finish up our frontend
    println!("Renderer:");
    let mut renderer = graphics::build(false, false).unwrap();

    {
        let frontend = frontend.lock().unwrap();
        match &frontend.info {
            &Some(ref v) => renderer.set_title(format!("OxRetro - {} ({})", v.library_name,
                                           v.library_version)),
            _ => panic!("Missing frontend info?")
        }
    }

    {
        frontend.lock().unwrap().renderer = Some(renderer);
    }


    println!("Audio:");
    {
        let audio = audio::build(av_info.timing.sample_rate as u32).unwrap();
        frontend.lock().unwrap().audio = Some(audio);
    }

    // Start up our main loop - we no longer need to talk to the frontend
    println!("Main loop!");
    loop {
        let (event, callback) = events.poll();
        match event {
            ProtocolMessageType::GetVariable(name) => callback(ProtocolMessageType::GetVariableResponse(None)),
            ProtocolMessageType::PollInput => frontend.lock().unwrap().poll_input(),
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
                match &mut frontend.lock().unwrap().renderer {
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
                match &mut frontend.lock().unwrap().renderer {
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
            },
            ProtocolMessageType::AudioSample(samples) => {
                match &mut frontend.lock().unwrap().audio {
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

    /*println!("Loading library...");
    let library = lib::Library::new("melonds_libretro.dll").unwrap();

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
    println!("{:?}", core.load_game(Some(Path::new("rom2.nds"))).unwrap());

    println!("Building context...");
    let mut renderer = graphics::build(false, false).unwrap();

    renderer.set_title(format!("OxRetro - {} ({})", frontend.info.library_name,
                               frontend.info.library_version));

    println!("Av:");
    let av_info = core.get_av_info().unwrap();

    println!("Endgame:");
    frontend.renderer = Some(renderer);

    let audio = audio::build(av_info.timing.sample_rate as u32).unwrap();

    frontend.audio = Some(audio);

    println!("Palette: {:?}", frontend.format);
    println!("Loop:");
    let max_frame = time::Duration::from_millis(16);

    while frontend.is_alive() {
        let start_loop = time::Instant::now();

        core.run().unwrap();

        frontend.variables_dirty = false;

        let elapsed = start_loop.elapsed();
        if elapsed < max_frame {
            let sleep_time = max_frame - elapsed;

            thread::sleep(sleep_time);
        }
    }

    println!("Core unload:");
    core.unload_game().unwrap();

    println!("Core deinit:");
    core.deinit().unwrap();

    println!("All done!");*/
}
