/// Implementation of the Backend<->Frontend communicator.

use backend::lib;
use backend::core::LibRetroCore;
use backend::state::BackendState;

use retro_types::RetroPixelFormat;

use core_protocol::ProtocolAdapter;
use core_protocol::ProtocolMessageType;
use core_protocol::ProtocolFuture;

use std::net::TcpStream;
use std::sync::Arc;
use std::sync::Mutex;
use std::path::Path;

static mut ADAPTER : Option<Arc<Mutex<ProtocolAdapter>>> = None;

/// Starts listening for messages over a socket.
pub fn run(core : String, port : u16) {
    println!("Loading library...");
    let library = lib::Library::new(&core).unwrap();

    println!("Configuring environment...");
    // TODO: RWLock would be much better! Do for all other mutexes as well
    let core = Arc::new(Mutex::new(LibRetroCore::from_library(library)));

    {
        core.lock().unwrap().configure_callbacks().unwrap();
    }

    let mut state = BackendState::new(RetroPixelFormat::Format0RGB1555);

    unsafe {
        state.make_current();
    }

    // TODO: Use alternate transport - ipc_channel doesn't support windows yet
    let stream = TcpStream::connect(format!("127.0.0.1:{}", port)).unwrap();

    let input = Box::new(stream.try_clone().unwrap());
    let output = Box::new(stream.try_clone().unwrap());
    // TODO: Consume events on the main thread
    let (comms, events) =
        ProtocolAdapter::new(input, output);

    // Store comms for later
    let boxed_comms = Arc::new(Mutex::new(comms));
    unsafe {
        ADAPTER = Some(boxed_comms);
    }

    loop {
        let (event, callback) = events.poll();

        let lock = core.lock().unwrap();

        // TODO: Error handling
        match event {
            ProtocolMessageType::Init => lock.init().unwrap(),
            ProtocolMessageType::Deinit => lock.deinit().unwrap(),
            ProtocolMessageType::Load(name) => assert!(lock.load_game(Some(Path::new(&name))).unwrap()),
            ProtocolMessageType::APIVersion => callback(ProtocolMessageType::APIVersionResponse(lock.get_api_version().unwrap())),
            ProtocolMessageType::Run => lock.run().unwrap(),
            ProtocolMessageType::Reset => lock.reset().unwrap(),
            ProtocolMessageType::SystemInfo => callback(ProtocolMessageType::SystemInfoResponse(lock.get_system_info().unwrap())),
            ProtocolMessageType::AVInfo => callback(ProtocolMessageType::AVInfoResponse(lock.get_av_info().unwrap())),
            _ => panic!("Unhandled command!")
        }
    }
}

/// Sends a message to the frontend, with a optional response.
pub fn send_message(message : ProtocolMessageType) -> Option<ProtocolFuture> {
    match unsafe { &ADAPTER } {
        &Some(ref v) => {
            v.lock().unwrap().send(message)
        },
        _ => panic!("No adapter to send message to!")
    }
}