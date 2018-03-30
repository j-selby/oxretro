/*!
This module contains function that will convert from one PCM format to another.

This includes conversion between samples formats, channels or sample rates.

*/
// Original source: https://github.com/tomaka/rodio/tree/master/src/conversions

pub use self::channels::ChannelsCountConverter;
pub use self::sample::DataConverter;
pub use self::sample::Sample;
pub use self::samples_rate::SamplesRateConverter;

mod channels;
mod sample;
mod samples_rate;
