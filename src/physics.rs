//! # Physics Layer
//!

const TEST_DATA: &str = "WHAT is truth? said jesting Pilate and would not stay for an answer. Certainly there be that delight";

pub const SAMPLE_RATE: f64 = 44100.0;

pub const CARRIER_FREQ: f64 = 441.0;

pub const SIGNAL_TIME: f64 = 0.05;

pub const PREAMBLE: f32 = 3.0;

use std::sync::Mutex;

use dasp::{signal, Signal};
use once_cell::sync::Lazy;

static ZERO_SIGNAL: Lazy<Mutex<Vec<f64>>> = Lazy::new(|| {
    Mutex::new(
        signal::rate(SAMPLE_RATE)
            .const_hz(CARRIER_FREQ)
            .phase()
            .sine()
            .take((SAMPLE_RATE * SIGNAL_TIME) as usize)
            .collect::<Vec<f64>>(),
    )
});

pub fn modulate_bit(b: u8) -> Vec<f64> {
    match b {
        0 => ZERO_SIGNAL.lock().unwrap().clone(),
        1 => ZERO_SIGNAL
            .lock()
            .unwrap()
            .iter()
            .map(|x| -x)
            .collect::<Vec<f64>>(),
        _ => panic!("only 0 and 1 are allowed"),
    }
}

pub fn demodulate_bit(fs: Vec<f32>) -> u8 {
    let b: f64 = fs
        .into_iter()
        .zip(ZERO_SIGNAL.lock().unwrap().clone())
        .map(|(x, y)| x as f64 + y)
        .sum();
    match b > 5.0 {
        true => 1,
        false => 0,
    }
}