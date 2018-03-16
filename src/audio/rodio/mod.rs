extern crate rodio;

use self::rodio::Sink;
use self::rodio::buffer::SamplesBuffer;

use audio::AudioBackend;
use audio::AudioBackendInfo;

pub struct RodioBackend {
    sink : Sink,
    sample_rate : u32
}

impl AudioBackend for RodioBackend {
    fn submit_frame(&mut self, frames: &[i16]) {
        let mut frames = frames.to_owned();
        // TODO: Do audio sanitation elsewhere
        for i in 0 .. frames.len() {
            frames[i] /= 4;
        }

        let buffer = SamplesBuffer::new(2,
                                        self.sample_rate, frames.to_owned());
        self.sink.append(buffer);
        self.sink.play();
    }

    fn is_done(&self) -> bool {
        self.sink.empty()
    }
}

pub fn build(sample_rate : u32) -> Box<AudioBackend> {
    let endpoint = rodio::default_endpoint().unwrap();
    let sink = Sink::new(&endpoint);

    Box::new(
        RodioBackend {
            sink,
            sample_rate
        }
    )
}

pub static INFO : AudioBackendInfo = AudioBackendInfo {
    name: "Rodio"
};

