/// Describes and implements the protocol used between the frontend and cores over IPC.

use std::borrow::BorrowMut;

use std::io::Read;
use std::io::Write;

use std::thread;

use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc;

use retro_types::RetroSystemInfo;

use bincode::{deserialize, serialize};

use byteorder::{ReadBytesExt, WriteBytesExt, LittleEndian};

use std::collections::HashMap;

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
    /// Environment command.
    Environment {
        /// If this environment call should block (i.e we want a response).
        blocking : bool,
    },
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

    // Frontend -> Backend messages
    /// Informs the core to warmup.
    Init,
    /// Informs the core to shutdown.
    Deinit,
    /// Returns the API version from the core. Blocking.
    APIVersion,
    /// Informs the core to run for a frame.
    Run,
    /// Informs the core to reset the application from the beginning.
    Reset,
    /// Returns the current core information. Blocking.
    SystemInfo,
    /// A response that should be inserted into the referenced memory.
    /// This is a raw array as we can append length to a Vec, but raw memory (used in C->Rust
    /// comms) coming from original environment calls has variable length.
    EnvironmentResponse(Vec<u8>),
    /// A response for what the current input state is.
    InputResponse(i16)
}

impl ProtocolMessageType {
    /// Returns if this message should be blocked on.
    pub fn is_blocking(&self) -> bool {
        match self {
            &ProtocolMessageType::Environment { blocking, .. } => blocking,
            &ProtocolMessageType::InputState { .. } => true,
            &ProtocolMessageType::APIVersion { .. } => true,
            &ProtocolMessageType::SystemInfo { .. } => true,
            _ => false
        }
    }

    /// Returns if this message is a response to something, and should be thrown through
    /// a callback.
    pub fn is_response(&self) -> bool {
        match self {
            &ProtocolMessageType::EnvironmentResponse( .. ) => true,
            &ProtocolMessageType::InputResponse( .. ) => true,
            &ProtocolMessageType::SystemInfoResponse( .. ) => true,
            &ProtocolMessageType::APIVersionResponse( .. ) => true,
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

impl ProtocolMessage {
    /// Returns if this message should be blocked on.
    pub fn is_blocking(&self) -> bool {
        self.data.is_blocking()
    }

    /// Returns if this message is a response to something, and should be thrown through
    /// a callback.
    pub fn is_response(&self) -> bool {
        self.data.is_response()
    }
}

/// A polling future where the future of the result can be found.
pub struct ProtocolFuture {
    receiver: Receiver<ProtocolMessageType>,
    already_recv: bool
}

impl ProtocolFuture {
    /// Tries to get a response, None otherwise. Doesn't block.
    pub fn try(&mut self) -> Option<ProtocolMessageType> {
        if self.already_recv {
            panic!("Already fetched a future!");
        }

        match self.receiver.try_recv() {
            Ok(v) => {
                self.already_recv = true;
                Some(v)
            },
            Err(_) => None
        }
    }

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

/// A server (which can run on either end) which handles I/O.
pub struct ProtocolAdapter {
    outgoing_tx : Sender<(Sender<ProtocolMessageType>, ProtocolMessageType, Option<u64>)>
}

/// Used internally for a select operation.
enum HandlingMessage {
    IncomingPacket(ProtocolMessage),
    /// A packet ready for transmission.
    /// Option<u64>: Forces the packet ID to be something (i.e for something which
    ///              demands a response).
    OutgoingPacket((Sender<ProtocolMessageType>,
                    ProtocolMessageType, Option<u64>))
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
    pub fn new(mut input : Box<Read + Send>, mut output : Box<Write + Send>,
        on_receive : fn(&ProtocolMessageType) -> Option<ProtocolMessageType>) -> ProtocolAdapter {
        // -- Socket handling
        // Packets that are incoming get their own decode thread
        let (decode_tx, decode_rx): (Sender<ProtocolMessage>,
                                     Receiver<ProtocolMessage>) = mpsc::channel();

        thread::spawn(move || {
            // Handle incoming packets
            loop {
                let input: &mut Read = &mut input;
                // TODO: Actually handle errors
                let length = input.read_u64::<LittleEndian>().unwrap();
                let mut data = vec![0 as u8; length as usize];
                input.read_exact(&mut data).unwrap();

                let packet : ProtocolMessage = deserialize(&data).unwrap();
                decode_tx.send(packet).unwrap();
            }
        });

        // decode_rx is now the interface to receive packets

        // Packets to be encoded and sent get their own thread
        let (encode_tx, encode_rx): (Sender<ProtocolMessage>,
                                     Receiver<ProtocolMessage>) = mpsc::channel();

        thread::spawn(move || {
            // Handle outgoing threads
            for i in encode_rx.iter() {
                let output: &mut Write = &mut output;
                // TODO: Actually handle errors
                let mut data = serialize(&i).unwrap();

                output.write_u64::<LittleEndian>(data.len() as u64).unwrap();
                output.write_all(&mut data).unwrap();
            }
        });

        // encode_tx is now the interface to send packets

        // -- Event loop
        // Packets that are outgoing get a channel to the loop
        let (outgoing_tx, outgoing_rx): (Sender<(Sender<ProtocolMessageType>,
                                                 ProtocolMessageType, Option<u64>)>,
                                         Receiver<(Sender<ProtocolMessageType>,
                                                   ProtocolMessageType, Option<u64>)>) = mpsc::channel();

        // Packets coming from the handler to this adapter
        let (incoming_tx, incoming_rx): (Sender<ProtocolMessage>,
                                         Receiver<ProtocolMessage>) = mpsc::channel();

        // Event loop
        let callback_outgoing_tx = outgoing_tx.clone();

        thread::spawn(move || {
            let mut packet_counter : u64 = 0;

            let mut callbacks : HashMap<u64, Sender<ProtocolMessageType>> = HashMap::new();

            loop {
                // Bit of wrapping to be able to use the match operator effectively
                let action : HandlingMessage = select! {
                    incoming_packet = decode_rx.recv() => {
                        HandlingMessage::IncomingPacket(incoming_packet.unwrap())
                    },
                    outgoing_packet = outgoing_rx.recv() => {
                        HandlingMessage::OutgoingPacket(outgoing_packet.unwrap())
                    }
                };

                match action {
                    HandlingMessage::IncomingPacket(packet) => {
                        if packet.is_response() {
                            // Call a specified callback handler
                            callbacks[&packet.id].send(packet.data).unwrap();
                        } else {
                            // Call the on_receive method for generic packets
                            let response = on_receive(&packet.data);
                            match response {
                                Some(v) => {
                                    // Create a dud callback - we don't support callbacks in callbacks
                                    let (null_tx, null_rx): (Sender<ProtocolMessageType>,
                                                             Receiver<ProtocolMessageType>) = mpsc::channel();

                                    // Send our response
                                    callback_outgoing_tx.send((null_tx, v, Some(packet.id))).unwrap();
                                },
                                _ => {}
                            }
                        }
                    },
                    HandlingMessage::OutgoingPacket((channel, packet, id)) => {
                        // Allocate a packet ID for this packet
                        let packet_id = match id {
                            Some(v) => v,
                            _ => {
                                let packet_id = packet_counter;
                                packet_counter += 1;
                                packet_counter
                            }
                        };

                        // Insert our callback if needed
                        if packet.is_blocking() {
                            callbacks.insert(packet_id, channel);
                        }

                        // Build our main structure
                        let final_packet = ProtocolMessage {
                            id: packet_id,
                            data: packet
                        };

                        // Send the packet to its destination
                        // TODO: Handle errors
                        encode_tx.send(final_packet).unwrap();
                    }
                }
            }
        });

        ProtocolAdapter {
            outgoing_tx
        }
    }
}
