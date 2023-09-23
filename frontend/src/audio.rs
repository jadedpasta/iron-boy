// SPDX-License-Identifier: GPL-3.0-or-later
// Copyright (C) 2023 Robert Hrusecky <jadedpastabowl@gmail.com>
use std::{error::Error, f32, sync::Arc};

use cpal::{
    traits::{DeviceTrait, HostTrait, StreamTrait},
    BufferSize, Device, FromSample, Sample, SampleFormat, SizedSample, Stream, StreamConfig,
    SupportedBufferSize,
};

use dasp::{
    interpolate::{linear::Linear, Interpolator},
    Frame as DaspFrame,
};

use crossbeam_queue::ArrayQueue;
use iron_boy_core::system::MachineCycle;

const CHANNELS: u16 = 2;
const ALPHA: f64 = 0.0001;
const BEND_CENTS: f64 = 3.0;
const BUFFER_SIZE: u32 = 256;
const SAMPLES_PER_M_CYCLE: usize = 2;
const FREQ: usize = MachineCycle::FREQ * SAMPLES_PER_M_CYCLE;
const SAMPLES_PER_FRAME: usize = MachineCycle::PER_FRAME * SAMPLES_PER_M_CYCLE;
const NAT_CUT_OFF_FREQ: f32 = 2.0 * f32::consts::PI * 4000.0;

type Frame = [f32; 2];

fn new_stream<T>(
    device: &Device,
    config: &StreamConfig,
    queue: &Arc<ArrayQueue<Frame>>,
) -> Result<Stream, Box<dyn Error>>
where
    T: SizedSample + FromSample<f32>,
{
    let sample_rate = config.sample_rate.0 as f32;
    let mut low_pass = Frame::EQUILIBRIUM;
    let low_pass_alpha = 1.0 / (sample_rate / NAT_CUT_OFF_FREQ + 1.0);

    let err_fn = |err| eprintln!("an error occurred on audio stream: {}", err);
    let queue = Arc::clone(queue);
    let stream = device.build_output_stream(
        config,
        move |output: &mut [T], _| {
            // println!("buf: {}", output.len() / 2);
            for frame in output.chunks_mut(CHANNELS as usize) {
                let value = queue.pop().unwrap_or(DaspFrame::EQUILIBRIUM);
                for ((output, input), low_pass) in
                    frame.iter_mut().zip(value).zip(low_pass.iter_mut())
                {
                    *low_pass += (input - *low_pass) * low_pass_alpha;
                    // *output = input.to_sample();
                    *output = low_pass.to_sample();
                }
            }
        },
        err_fn,
        None,
    )?;
    stream.play()?;

    Ok(stream)
}

struct Resampler<I> {
    interpolator: I,
    ratio: f64,    // target hz / source hz
    progress: f64, // { n * ratio }
}

impl<F> Resampler<Linear<F>>
where
    F: DaspFrame,
{
    fn new(ratio: f64) -> Self {
        Self {
            interpolator: Linear::new(F::EQUILIBRIUM, F::EQUILIBRIUM),
            ratio,
            progress: 0.0,
        }
    }
}

impl<I> Resampler<I>
where
    I: Interpolator,
{
    fn push_frame(&mut self, source: I::Frame, sink: &Arc<ArrayQueue<I::Frame>>) {
        self.interpolator.next_source_frame(source);
        self.progress += self.ratio;

        while self.progress >= 1.0 {
            self.progress -= 1.0;
            let x = 1.0 - self.progress / self.ratio;
            let _ = sink.push(self.interpolator.interpolate(x));
        }
    }
}

pub struct Audio {
    _stream: Stream, // We have to keep the stream alive to keep sound playing
    queue: Arc<ArrayQueue<Frame>>,
    resampler: Resampler<Linear<Frame>>,
    min_ratio: f64,
    max_ratio: f64,
    average_len: f64,
    push_count: usize,
}

impl Audio {
    pub fn update_ratio(&mut self) {
        // println!("push_count: {}/{}", self.push_count, MachineCycle::PER_FRAME);
        self.push_count = 0;
        let len = self.queue.len();
        if len > 0 {
            // Low-pass filter on the queue length
            // println!("avg: {}, curr: {}", self.average_len, len);
            self.average_len += (len as f64 - self.average_len) * ALPHA;
        } else {
            // HACK: Shove some samples in there to get the queue to the expected len
            for _ in 0..(self.average_len / self.resampler.ratio) as usize {
                self.push_frame(DaspFrame::EQUILIBRIUM);
            }
            // println!("hack: {}, {}", self.average_len, self.queue.len());
        }

        let ratio =
            (self.queue.capacity() as f64 / 2.0 - self.average_len) / (SAMPLES_PER_FRAME as f64);
        self.resampler.ratio = ratio.clamp(self.min_ratio, self.max_ratio);
        // println!("ratio: {}", self.resampler.ratio);
    }

    pub fn push_frame(&mut self, frame: Frame) {
        self.push_count += 1;
        self.resampler.push_frame(frame, &self.queue);
    }
}

pub fn init() -> Result<Audio, Box<dyn Error>> {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .ok_or("No output device found")?;
    let default_config = device.default_output_config()?;
    let sample_format = default_config.sample_format();
    let sample_rate = default_config.sample_rate();

    let config = device
        .supported_output_configs()?
        .find(|r| {
            if let SupportedBufferSize::Range { min, max } = *r.buffer_size() {
                r.channels() == CHANNELS
                    && r.sample_format() == sample_format
                    && sample_rate >= r.min_sample_rate()
                    && sample_rate <= r.max_sample_rate()
                    && BUFFER_SIZE >= min
                    && BUFFER_SIZE <= max
            } else {
                false
            }
        })
        .ok_or("Could find acceptable audio configuration")?
        .with_sample_rate(sample_rate);

    let config = StreamConfig {
        buffer_size: BufferSize::Fixed(BUFFER_SIZE),
        ..config.into()
    };

    // println!("Audio stream config: {config:#?}");

    let sample_rate = config.sample_rate.0 as f64;

    let len = (sample_rate / 10.0) as usize;
    let queue = Arc::new(ArrayQueue::<Frame>::new(len));

    let stream = match sample_format {
        SampleFormat::F32 => new_stream::<f32>(&device, &config, &queue),
        SampleFormat::I16 => new_stream::<i16>(&device, &config, &queue),
        SampleFormat::U16 => new_stream::<u16>(&device, &config, &queue),
        SampleFormat::U8 => new_stream::<u8>(&device, &config, &queue),
        sample_format => Err(format!("Unsupported sample format '{sample_format}'").into()),
    }?;

    let ratio = sample_rate / FREQ as f64;
    let fps = MachineCycle::FREQ as f64 / MachineCycle::PER_FRAME as f64;

    // println!("initial avg: {}", queue.capacity() as f64 / 2.0 - sample_rate / fps);

    let audio = Audio {
        push_count: 0,
        _stream: stream,
        average_len: queue.capacity() as f64 / 2.0 - sample_rate / fps,
        queue,
        resampler: Resampler::new(ratio),
        max_ratio: ratio * 2f64.powf(BEND_CENTS / 1200.0),
        min_ratio: ratio * 2f64.powf(-BEND_CENTS / 1200.0),
    };

    Ok(audio)
}
