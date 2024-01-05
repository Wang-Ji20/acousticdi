use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Sample, SampleRate};
use dasp::sample::ToSample;
use tracing::{error, info};

use crate::output_wav;
use crate::transmission::SampleReader;

type BufferHandle = Arc<Mutex<Vec<f32>>>;

#[derive(Debug)]
pub struct Recorder {
    ring_buffer: BufferHandle,
}

impl Default for Recorder {
    fn default() -> Self {
        Self::new()
    }
}

impl Recorder {
    pub fn new() -> Recorder {
        Recorder {
            ring_buffer: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub fn clone_handle(&mut self) -> BufferHandle {
        self.ring_buffer.clone()
    }

    pub fn take_samples(&mut self, start: usize, end: usize) -> Vec<f64> {
        while self.ring_buffer.lock().unwrap().len() < end {}
        let ring_buffer = self.ring_buffer.lock().unwrap();
        ring_buffer[start..end].iter().map(|f| *f as f64).collect()
    }

    pub fn save_to_wav(&mut self) {
        output_wav(
            &self
                .ring_buffer
                .lock()
                .unwrap()
                .clone()
                .iter()
                .map(|f| *f as f64)
                .collect::<Vec<f64>>(),
            "recorder.wav",
        )
    }
}

#[test]
fn test_recorder() {
    let _ = tracing_subscriber::fmt::try_init();
    let mut recorder = Recorder::new();
    let _stream = run_record(recorder.clone_handle()).unwrap();
    sleep(Duration::from_secs(3));
    recorder.save_to_wav();
}

impl SampleReader for Recorder {
    fn take_samples(&mut self, start: usize, end: usize) -> Vec<f64> {
        self.take_samples(start, end)
    }
}

/// start record and analysis routines.
///
/// NB: The returned `Stream` is RAII guarded, so the caller should not drop it until
/// recording finishes.
pub fn run_record(handle: BufferHandle) -> Result<cpal::Stream, anyhow::Error> {
    info!("run record.. preparing");
    let host = cpal::default_host();

    // Set up the input device and stream with the default input config.
    let device = host
        .default_input_device()
        .expect("failed to find input device");

    info!("Input device: {}", device.name()?);

    let configs = device
        .supported_input_configs()
        .expect("Failed to get default input config");

    let mut config = device.default_input_config().unwrap();

    for cfg in configs {
        if cfg.channels() == 1 {
            config = cfg.with_sample_rate(SampleRate(44100));
        }
    }

    println!("config: {:?}", config);

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
