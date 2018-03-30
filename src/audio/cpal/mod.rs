extern crate cpal;

mod conversions;

//use self::cpal::Sink;
//use self::cpal::buffer::SamplesBuffer;

use audio::AudioBackend;
use audio::AudioBackendInfo;

use std::sync::mpsc::Sender;
use std::sync::mpsc;
use std::sync::mpsc::Receiver;
use std::thread;
use std::sync::Arc;
use std::sync::Mutex;

use self::conversions::*;

use std::u16::MAX as u16_max;
use std::i16::MAX as i16_max;

/// Structure for storing count of audio remaining
struct RemainingAudio {
    frames : i64
}

/// Counts audio passing through
struct AudioCounter<I>
    where I: Iterator {
    counter : Arc<Mutex<RemainingAudio>>,
    incoming : I
}

impl<I> Iterator for AudioCounter<I>
    where I: Iterator {
    type Item = I::Item;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        self.counter.lock().unwrap().frames -= 1;
        self.incoming.next()
    }
}

pub struct RodioBackend {
    sink : Sender<i16>,
    samples_remaining : Arc<Mutex<RemainingAudio>>,
    sample_rate : u32
}

impl AudioBackend for RodioBackend {
    fn submit_frame(&mut self, frames: &[i16]) {
        let mut frames = frames.to_owned();
        // TODO: Do audio sanitation elsewhere
        for i in 0 .. frames.len() {
            frames[i] /= 4;
        }

        let mut sample_size = self.samples_remaining.lock().unwrap();

        for i in 0 .. frames.len() {
            self.sink.send(frames[i]).unwrap();
        }

        sample_size.frames += frames.len() as i64;
    }

    fn is_done(&self) -> bool {
        // TODO: Don't hardcode framerate
        self.samples_remaining.lock().unwrap().frames < self.sample_rate as i64 / 60 * 2
    }

    fn get_done_callback(&self) -> Box<Fn() -> bool + Send> {
        let inner_samples = self.samples_remaining.clone();
        let sample_rate = self.sample_rate;
        Box::new(move || {
            inner_samples.lock().unwrap().frames < sample_rate as i64 / 60 * 2
        })
    }
}

pub fn build(sample_rate : u32) -> Box<AudioBackend> {
    let (frame_tx, frame_rx): (Sender<i16>,
                               Receiver<i16>) = mpsc::channel();

    let sample_mutex = Arc::new(Mutex::new(RemainingAudio {
        frames: 0
    }));

    let thread_mutex = sample_mutex.clone();

    thread::Builder::new().name("cpal-audio".to_owned()).spawn(move || {

        let device = cpal::default_output_device().expect("Failed to get default output device");
        let format = device.default_output_format().expect("Failed to get default output format");
        let target_sample_rate = format.sample_rate;
        let sample_channels = format.channels;

        let counter = AudioCounter {
            counter: thread_mutex,
            incoming: frame_rx.into_iter(),
        };

        let mut converter = DataConverter::new(
            ChannelsCountConverter::new(
                SamplesRateConverter::new(counter, cpal::SampleRate(sample_rate),
                                          target_sample_rate, 2),
                2, sample_channels));

        let event_loop = cpal::EventLoop::new();
        let stream_id = event_loop.build_output_stream(&device, &format).unwrap();
        event_loop.play_stream(stream_id.clone());

        event_loop.run(move |_, data| {
            match data {
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::U16(mut buffer) } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        for out in sample.iter_mut() {
                            let sample : f32 = converter.next().unwrap();
                            *out = ((sample * 0.5 + 0.5) * u16_max as f32) as u16;
                        }
                    }
                },
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::I16(mut buffer) } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        for out in sample.iter_mut() {
                            let sample : f32 = converter.next().unwrap();
                            *out = (sample * i16_max as f32) as i16;
                        }
                    }
                },
                cpal::StreamData::Output { buffer: cpal::UnknownTypeOutputBuffer::F32(mut buffer) } => {
                    for sample in buffer.chunks_mut(format.channels as usize) {
                        for out in sample.iter_mut() {
                            let sample : f32 = converter.next().unwrap();
                            *out = sample;
                        }
                    }
                },
                _ => (),
            }
        });


        /*let incoming = Vec::new();

        loop {
            let v = match frame_rx.recv() {
                Ok(v) => v,
                Err(_) => break
            };

            samples.lock().unwrap().frames += v.len() as u64;
            let buffer = SamplesBuffer::new(2,
                                            sample_rate, v);
            sink.append(buffer);
            sink.play();
        }*/
    }).unwrap();

    Box::new(
        RodioBackend {
            sink : frame_tx,
            samples_remaining : sample_mutex,
            sample_rate
        }
    )
}

pub static INFO : AudioBackendInfo = AudioBackendInfo {
    name: "Rodio"
};

