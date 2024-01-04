//! # Physics Layer
//!
//! We use six frequencies to encode the data. One signal per six bits.

const TEST_DATA: &str = "WHAT is truth? said jesting Pilate and would not stay for an answer. Certainly there be that delight";

pub const SAMPLE_RATE: f64 = 44100.0;

pub const FREQ_NUMBER: usize = 4;

pub const CARRIER_FREQS: [f64; FREQ_NUMBER] = [
    2083.464566929134,
    2604.3307086614172,
    3472.4409448818897,
    4166.929133858268,
];

pub const SIGNAL_TIME: f64 = 0.2;

pub const SAMPLE_NUMBER: usize = (SAMPLE_RATE * SIGNAL_TIME) as usize;

pub const PREAMBLE: f32 = 3.0;

use std::{iter::repeat, sync::Mutex};

use dasp::{signal, Signal};
use once_cell::sync::Lazy;
use ruststft::STFT;
use tracing::info;

type AudioSignal = Vec<f64>;
type AudioSignalHandle = Lazy<Mutex<Vec<AudioSignal>>>;

static SIGNALS: AudioSignalHandle = Lazy::new(|| {
    Mutex::new(
        CARRIER_FREQS
            .iter()
            .map(|freq| {
                signal::rate(SAMPLE_RATE)
                    .const_hz(*freq)
                    .phase()
                    .sine()
                    .take(SAMPLE_NUMBER)
                    .collect::<Vec<f64>>()
            })
            .collect(),
    )
});

static FFT_FREQS: Lazy<Mutex<Vec<f64>>> = Lazy::new(|| {
    use ruststft::STFT;
    let stft: STFT<f64> = STFT::new(ruststft::WindowType::Hanning, 256, 128);
    Mutex::new(stft.freqs(SAMPLE_RATE))
});

#[test]
fn test_freqs() {
    use crate::output_wav;
    for i in 0..FREQ_NUMBER {
        output_wav(&SIGNALS.lock().unwrap()[i], &format!("{}.wav", i))
    }
}

#[test]
fn test_add() {
    use crate::output_wav;
    let a = SIGNALS.lock().unwrap()[0].clone();
    let b = vector_add(&a, &SIGNALS.lock().unwrap()[3])
        .iter()
        .map(|x| x / 2.0)
        .collect::<Vec<f64>>();

    output_wav(&b, "01.wav");

    let mut stft = ruststft::STFT::new(ruststft::WindowType::Hanning, 256, 128);
    let result = stft_result(&mut stft, &b);
    println!(
        "{:?}, {}",
        result[10]
            .iter()
            .map(|x| 10.0_f64.powf(*x))
            .collect::<Vec<f64>>(),
        result[10].len()
    );
}

pub fn modulate_bits(b: Vec<u8>) -> Vec<f64> {
    b.iter()
        .flat_map(|b| modulate_byte(*b))
        .collect::<Vec<f64>>()
}

pub fn modulate_byte(b: u8) -> Vec<f64> {
    let higher_four = (b & 0b11110000) >> 4;
    let lower_four = b & 0b00001111;
    [modulate_bit(higher_four), modulate_bit(lower_four)].concat()
}

pub fn modulate_bit(b: u8) -> Vec<f64> {
    let mut modulate_result: Vec<f64> = repeat(0.0).take(SAMPLE_NUMBER).collect();
    let mut normalize_factor = 0;
    for i in 0..FREQ_NUMBER {
        if (b & (1_u8 << i)) > 0 {
            info!("add {}th bit", i);
            modulate_result = vector_add(&modulate_result, &SIGNALS.lock().unwrap()[i]);
            normalize_factor += 1;
        }
    }
    modulate_result
        .into_iter()
        .map(|x| {
            x / (match normalize_factor {
                0 => 1.0,
                normalize => normalize as f64,
            })
        })
        .collect()
}

fn vector_add(v1: &[f64], v2: &[f64]) -> Vec<f64> {
    v1.iter().zip(v2.iter()).map(|(x, y)| *x + *y).collect()
}

fn stft_result(stft: &mut STFT<f64>, input: &[f64]) -> Vec<Vec<f64>> {
    let mut result = Vec::new();
    stft.append_samples(input);
    while stft.contains_enough_to_compute() {
        let out_size = stft.output_size();
        let mut segment_result = repeat(0.0).take(out_size).collect::<Vec<f64>>();
        stft.compute_column(&mut segment_result);
        result.push(segment_result);
        stft.move_to_next_column();
    }
    result
}

#[test]
fn test_modulate_byte() {
    tracing_subscriber::fmt::init();
    let x = 0b00110011;
    let modulated = modulate_byte(x);
    let mut stft = ruststft::STFT::new(ruststft::WindowType::Hanning, 256, 128);
    let result = stft_result(&mut stft, &modulated);
    println!("{:?}, {}", result[5], result[5].len());
    let b = demodulate_byte(&mut stft, modulated);
    println!("{:#b}", b);
}

pub fn demodulate_byte(stft: &mut STFT<f64>, fs: Vec<f64>) -> u8 {
    let result = stft_result(stft, &fs);
    let freq_col = result[result.len() / 2]
        .iter()
        .map(|x| 10.0_f64.powf(*x))
        .collect::<Vec<f64>>();
    let mut freq_col_idx: Vec<(f64, usize)> = freq_col.into_iter().zip(0..).collect();
    freq_col_idx.sort_by(|(x, _), (a, _)| x.partial_cmp(a).unwrap());
    freq_col_idx.reverse();
    let mut prev_energy = freq_col_idx[0].0;
    let mut freqs = vec![];
    for (energy, idx) in freq_col_idx {
        if (energy - prev_energy).abs() > 3.0 {
            break;
        }
        prev_energy = energy;
        freqs.push(FFT_FREQS.lock().unwrap()[idx]);
    }
    let mut byte_result = 0_u8;
    for freq in freqs {
        info!("detected freq {}", freq);
        let idx = CARRIER_FREQS
            .binary_search_by(|probe| probe.partial_cmp(&freq).unwrap())
            .unwrap();
        byte_result |= 1 << idx;
    }
    byte_result
}

#[test]
fn test_stft() {
    use ruststft::STFT;
    let test_signal = SIGNALS.lock().unwrap()[0].clone();
    let mut sfft: STFT<f64> = STFT::new(ruststft::WindowType::Hanning, 256, 128);
    sfft.append_samples(&test_signal);
    let mut result = repeat(0.0).take(sfft.output_size()).collect::<Vec<f64>>();
    sfft.compute_column(&mut result);
    println!("{:?}, {}", sfft.freqs(44100.0), sfft.freqs(44100.0).len());
}
