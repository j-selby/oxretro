/// Describes and implements the protocol used between the frontend and cores over IPC.

use std::io::Read;
use std::io::Write;

use std::thread;

use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc;
use std::sync::Arc;
use std::sync::Mutex;

use std::collections::HashMap;

use std::error::Error;

use retro_types::RetroSystemInfo;
use retro_types::RetroVariable;
use retro_types::RetroAvInfo;

use bincode::{deserialize, serialize};

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};

/// Defines the various types of video refresh that can occur.
#[derive(Deserialize, Serialize)]
pub enum VideoRefreshType {
    /// A software refresh is where a core rasterises images on its own, and
    /// sends it as a byte array.
    Software {
        framebuffer : Vec<u8>,
        width : u64,
        height : u64
    },
    /// A hardware refresh is when the image is already on the GPU (i.e a
    /// OpenGL framebuffer).
    Hardware
}

/// Contains data used to hold various messages.
#[derive(Deserialize, Serialize)]
pub enum ProtocolMessageType {
    // Backend -> Frontend messages
    /// Set variables for the frontend.
    SetVariables(Vec<RetroVariable>),
    /// Returns a particular variable setting. Blocking.
    GetVariable(String),
    /// Called when the core is ready to submit an audio frame.
    VideoRefresh(VideoRefreshType),
    /// Core submitting >=1 audio samples.
    AudioSample(Vec<i16>),
    /// Core requesting that input be updated on the frontend.
    PollInput,
    /// Core asking for the current state of a particular input mechanism. Blocking.
    InputState {
        port : u32,
        device : u32,
        index : u32,
        id : u32
    },
    /// A response to a System Info query.
    SystemInfoResponse(RetroSystemInfo),
    /// A response to an API version query.
    APIVersionResponse(u32),
    /// A response to an A/V query.
    AVInfoResponse(RetroAvInfo),
    /// A response to a run query.
    RunResponse,

    // Frontend -> Backend messages
    /// Informs the core to warmup.
    Init,
    /// Informs the core to shutdown.
    Deinit,
    /// Informs the core to load something.
    Load(String),
    /// Informs the core to unload.
    Unload,
    /// Returns the API version from the core. Blocking.
    APIVersion,
    /// Returns the A/V info for this core. Blocking.
    AVInfo,
    /// Informs the core to run for a frame. Blocking.
    Run,
    /// Informs the core to reset the application from the beginning.
    Reset,
    /// Returns the current core information. Blocking.
    SystemInfo,
    /// A response for what the current input state is.
    InputResponse(i16),
    /// Returns a value contained within a variable.
    GetVariableResponse(Option<String>)
}

impl ProtocolMessageType {
    /// Returns if this message should be blocked on.
    pub fn is_blocking(&self) -> bool {
        match self {
            &ProtocolMessageType::InputState { .. } => true,
            &ProtocolMessageType::APIVersion { .. } => true,
            &ProtocolMessageType::SystemInfo { .. } => true,
            &ProtocolMessageType::GetVariable { .. } => true,
            &ProtocolMessageType::AVInfo { .. } => true,
            &ProtocolMessageType::Run => true,
            _ => false
        }
    }

    /// Returns if this message is a response to something, and should be thrown through
    /// a callback.
    pub fn is_response(&self) -> bool {
        match self {
            &ProtocolMessageType::InputResponse( .. ) => true,
            &ProtocolMessageType::SystemInfoResponse( .. ) => true,
            &ProtocolMessageType::APIVersionResponse( .. ) => true,
            &ProtocolMessageType::GetVariableResponse( .. ) => true,
            &ProtocolMessageType::AVInfoResponse( .. ) => true,
            &ProtocolMessageType::RunResponse => true,
            _ => false
        }
    }
}

/// Contains protocol overhead.
#[derive(Deserialize, Serialize)]
pub struct ProtocolMessage {
    /// Used to uniquely identify a message. Can be used in a response
    /// for two way communication.
    id : u64,
    data : ProtocolMessageType,
}

/// A polling future where the future of the result can be found.
pub struct ProtocolFuture {
    receiver: Receiver<ProtocolMessageType>,
    already_recv: bool
}

impl ProtocolFuture {
    /// Polls for a response. Panics if value unable to be received.
    pub fn poll(&mut self) -> ProtocolMessageType {
        if self.already_recv {
            panic!("Already fetched a future!");
        }

        self.already_recv = true;
        self.receiver.recv().unwrap()
    }

    /// Unwraps this future with the given response. Panics if value unable to be received.
    pub fn unwrap(mut self) -> ProtocolMessageType {
        self.poll()
    }
}

/// Interface to handle incoming events from a ProtocolAdapter
pub struct ProtocolEvents {
    incoming_rx : Receiver<ProtocolMessage>,
    outgoing_tx : Sender<(Sender<ProtocolMessageType>, ProtocolMessageType, Option<u64>)>
}

impl ProtocolEvents {
    /// Receives an incoming event. Blocking.
    /// Returns: message, optional handler to reply.
    pub fn poll(&self) -> Option<(ProtocolMessageType, Box<Fn(ProtocolMessageType)>)> {
        let incoming = match self.incoming_rx.recv() {
            Ok(v) => v,
            Err(_) => {
                return None;
            }
        };

        let id = incoming.id;
        let cloned_tx = self.outgoing_tx.clone();

        Some((incoming.data, Box::new(move |message| {
            // Create a dud callback - we don't support callbacks in callbacks (yet)
            let (null_tx, _): (Sender<ProtocolMessageType>,
                                     Receiver<ProtocolMessageType>) = mpsc::channel();

            // Send our response
            cloned_tx.send((null_tx, message, Some(id))).unwrap();
        })))
    }
}

/// A server (which can run on either end) which handles I/O.
pub struct ProtocolAdapter {
    outgoing_tx : Sender<(Sender<ProtocolMessageType>, ProtocolMessageType, Option<u64>)>
}

impl ProtocolAdapter {
    /// Sends a message down the pipe.
    pub fn send(&self, message : ProtocolMessageType) -> Option<ProtocolFuture> {
        // Create channel for our future
        let (incoming_tx, incoming_rx): (Sender<ProtocolMessageType>,
                                         Receiver<ProtocolMessageType>) = mpsc::channel();

        let is_blocking = message.is_blocking();

        self.outgoing_tx.send((incoming_tx, message, None)).unwrap();

        if is_blocking {
            Some(ProtocolFuture {
                receiver : incoming_rx,
                already_recv : false
            })
        } else {
            None
        }
    }

    /// Creates a new protocol adapter with the specified input/output streams.
    /// on_receive: function called when a protocol message is received. Optional return
    ///             for a response to the client/server.
    pub fn new(name : String, mut input : Box<Read + Send>, mut output : Box<Write + Send>)
        -> (ProtocolAdapter, ProtocolEvents) {
        // -- Socket handling
        // Packets that are incoming get their own decode thread
        let (decode_tx, decode_rx): (Sender<ProtocolMessage>,
                                     Receiver<ProtocolMessage>) = mpsc::channel();

        let decode_thread_name = name.clone();
        thread::Builder::new().name(format!("{}-decode", name)).spawn(move || {
            // Handle incoming packets
            loop {
                let input: &mut Read = &mut input;

                let length = match input.read_u64::<LittleEndian>() {
                    Ok(v) => v,
                    Err(e) => {
                        println!("{} incoming data thread is shutting down: {}", decode_thread_name,
                                 e.description());
                        break;
                    }
                };
                let mut data = vec![0 as u8; length as usize];
                match input.read_exact(&mut data) {
                    Err(e) => {
                        println!("{} incoming data thread is shutting down: {}", decode_thread_name,
                                 e.description());
                        break;
                    },
                    _ => {}
                };

                let packet : ProtocolMessage = deserialize(&data).unwrap();
                decode_tx.send(packet).unwrap();
            }
        }).unwrap();

        // decode_rx is now the interface to receive packets

        // Packets to be encoded and sent get their own thread
        let (encode_tx, encode_rx): (Sender<ProtocolMessage>,
                                     Receiver<ProtocolMessage>) = mpsc::channel();

        let encode_thread_name = name.clone();
        thread::Builder::new().name(format!("{}-encode", name)).spawn(move || {
            // Handle outgoing threads
            for i in encode_rx.iter() {
                let output: &mut Write = &mut output;
                let mut data = serialize(&i).unwrap();

                let mut final_packet = Vec::new();
                final_packet.write_u64::<LittleEndian>(data.len() as u64).unwrap();
                final_packet.append(&mut data);

                match output.write_all(&mut final_packet) {
                    Err(e) => {
                        println!("{} outgoing data thread is shutting down: {}", encode_thread_name,
                                 e.description());
                        break;
                    },
                    _ => {}
                };
            }
        }).unwrap();

        // encode_tx is now the interface to send packets

        // -- Event loop
        // Packets that are outgoing get a channel to the loop
        let (outgoing_tx, outgoing_rx): (Sender<(Sender<ProtocolMessageType>,
                                                 ProtocolMessageType, Option<u64>)>,
                                         Receiver<(Sender<ProtocolMessageType>,
                                                   ProtocolMessageType, Option<u64>)>) = mpsc::channel();

        let callbacks : Arc<Mutex<HashMap<u64, Sender<ProtocolMessageType>>>> =
            Arc::new(Mutex::new(HashMap::new()));

        // Incoming event loop
        let callback_outgoing_tx = outgoing_tx.clone();

        // Packs sent from the event loop to the event handler
        let (callback_incoming_tx, callback_incoming_rx): (Sender<ProtocolMessage>,
                                                           Receiver<ProtocolMessage>) = mpsc::channel();

        let incoming_callbacks = callbacks.clone();
        thread::Builder::new().name(format!("{}-incoming", name)).spawn(move || {
            loop {
                let packet = match decode_rx.recv() {
                    Err(_) => break,
                    Ok(v) => v
                };

                if packet.data.is_response() {
                    // Call a specified callback handler
                    incoming_callbacks.lock().unwrap().remove(&packet.id).unwrap().send(packet.data).unwrap();
                } else {
                    // We don't want to block the event loop
                    callback_incoming_tx.send(packet).unwrap();
                }
            }
        }).unwrap();

        // Outgoing event loop
        let outgoing_callbacks = callbacks;
        thread::Builder::new().name(format!("{}-outgoing", name)).spawn(move || {
            let mut packet_counter : u64 = 0;

            loop {
                let (channel, packet,
                    id) = match outgoing_rx.recv() {
                    Err(_) => break,
                    Ok(v) => v
                };

                // Allocate a packet ID for this packet
                let packet_id = match id {
                    Some(v) => v,
                    _ => {
                        let packet_id = packet_counter;
                        packet_counter += 1;
                        packet_id
                    }
                };

                // Insert our callback if needed
                if packet.is_blocking() {
                    outgoing_callbacks.lock().unwrap().insert(packet_id, channel);
                }

                // Build our main structure
                let final_packet = ProtocolMessage {
                    id: packet_id,
                    data: packet
                };

                // Send the packet to its destination
                match encode_tx.send(final_packet) {
                    Err(_) => break,
                    _ => {}
                }
            }
        }).unwrap();

        let events = ProtocolEvents {
            incoming_rx: callback_incoming_rx,
            outgoing_tx: callback_outgoing_tx
        };

        let adapter = ProtocolAdapter {
            outgoing_tx
        };

        (adapter, events)
    }
}
