use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::Sample;
use dasp::sample::ToSample;
use tracing::{error, info};

type BufferHandle = Arc<Mutex<VecDeque<f32>>>;

#[derive(Debug)]
pub struct Recorder {
    ring_buffer: BufferHandle,
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder {
            ring_buffer: Arc::new(Mutex::new(VecDeque::new())),
        }
    }

    /// Take `samples` of recordings, erasing them from the buffer
    pub fn take_owned(&mut self, num_samples: usize) -> Vec<f32> {
        while self.ring_buffer.lock().unwrap().len() < num_samples {}
        let samples = self
            .ring_buffer
            .lock()
            .unwrap()
            .drain(0..num_samples)
            .collect();
        samples
    }

    pub fn clone_handle(&mut self) -> BufferHandle {
        self.ring_buffer.clone()
    }
}

/// start record and analysis routines.
pub fn run_record(handle: BufferHandle) -> Result<cpal::Stream, anyhow::Error> {
    info!("run record.. preparing");
    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = host
        .default_input_device()
        .expect("failed to find input device");

    info!("Input device: {}", device.name()?);

    let config = device
        .default_input_config()
        .expect("Failed to get default input config");
    info!("Default input config: {:?}", config);

    let err_fn = move |err| {
        error!("an error occurred on stream: {}", err);
    };

    let stream = match config.sample_format() {
        cpal::SampleFormat::I8 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i8>(data, handle.clone()),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i16>(data, handle.clone()),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::I32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<i32>(data, handle.clone()),
            err_fn,
            None,
        )?,
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data, _: &_| write_input_data::<f32>(data, handle.clone()),
            err_fn,
            None,
        )?,
        sample_format => {
            return Err(anyhow::Error::msg(format!(
                "Unsupported sample format '{sample_format}'"
            )))
        }
    };

    stream.play()?;

    // A flag to indicate that recording is in progress.
    info!("Begin recording...");

    // return to caller. This function has done everything.
    Ok(stream)
}

fn write_input_data<T>(input: &[T], handle: BufferHandle)
where
    T: Sample + ToSample<f32>,
{
    handle
        .lock()
        .unwrap()
        .extend(input.iter().map(|x| x.to_sample::<f32>()))
}
