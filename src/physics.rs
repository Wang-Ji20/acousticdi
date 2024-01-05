//! # Physics Layer
//!
//! We use six frequencies to encode the data. One signal per six bits.

pub const FREQ_NUMBER: usize = 4;

use crate::{
    output_wav,
    transmission::{SAMPLE_NUMBER, SAMPLE_RATE},
};

pub const CARRIER_FREQS: [f64; FREQ_NUMBER] = [
    2083.464566929134,
    2604.3307086614172,
    3472.4409448818897,
    4166.929133858268,
];

pub const PREAMBLE_NUMBER: usize = 2;

pub const PREAMBLE_FREQS: [f64; PREAMBLE_NUMBER] = [1388.976377952756, 2951.5748031496064];

static PREAMBLE_SIGNALS: AudioSignalHandle =
    Lazy::new(|| Mutex::new(generate_signals(&PREAMBLE_FREQS)));

pub const PREAMBLE_SEQUENCE: u8 = 0b01010101;

use std::{iter::repeat, sync::Mutex};

use dasp::{signal, Signal};
use once_cell::sync::Lazy;
use ruststft::STFT;
use tracing::info;

type AudioSignal = Vec<f64>;
type AudioSignalHandle = Lazy<Mutex<Vec<AudioSignal>>>;

static SIGNALS: AudioSignalHandle = Lazy::new(|| Mutex::new(generate_signals(&CARRIER_FREQS)));

static FFT_FREQS: Lazy<Mutex<Vec<f64>>> = Lazy::new(|| {
    let stft: ruststft::STFT<f64> = STFT::new(ruststft::WindowType::Hanning, 256, 128);
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

fn generate_signals(freqs: &[f64]) -> Vec<AudioSignal> {
    freqs
        .iter()
        .map(|freq| {
            signal::rate(SAMPLE_RATE)
                .const_hz(*freq)
                .phase()
                .sine()
                .take(SAMPLE_NUMBER)
                .collect()
        })
        .collect()
}

pub fn modulate_bits(b: Vec<u8>) -> Vec<f64> {
    b.iter()
        .flat_map(|b| modulate_byte(*b))
        .collect::<Vec<f64>>()
}

pub fn modulate_byte(b: u8) -> Vec<f64> {
    let higher_four = (b & 0b11110000) >> 4;
    let lower_four = b & 0b00001111;
    [
        modulate_half_byte(higher_four),
        modulate_half_byte(lower_four),
    ]
    .concat()
}

pub fn modulate_half_byte(b: u8) -> Vec<f64> {
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
    assert!(v1.len() == v2.len());
    v1.iter().zip(v2.iter()).map(|(x, y)| *x + *y).collect()
}

fn stft_result(stft: &mut STFT<f64>, input: &[f64]) -> Vec<Vec<f64>> {
    let mut result = Vec::new();
    stft.append_samples(input);
    while stft.contains_enough_to_compute() {
        let out_size = stft.output_size();
        let mut segment_result = repeat(0.0).take(out_size).collect::<Vec<f64>>();
        stft.compute_column(&mut segment_result);
        result.push(segment_result.iter().map(|f| 10.0_f64.powf(*f)).collect());
        stft.move_to_next_column();
    }
    result
}

#[test]
fn test_modulate_byte() {
    tracing_subscriber::fmt::init();
    let x = 0b00110111;
    let modulated = modulate_byte(x);
    let mut stft = ruststft::STFT::new(ruststft::WindowType::Hanning, 256, 128);
    let result = stft_result(&mut stft, &modulated);
    println!("{:?}, {}", result[5], result[5].len());
    let b = demodulate_half_byte(&mut stft, modulated.clone());
    let lower_b = demodulate_half_byte(&mut stft, modulated[modulated.len() / 2..].to_vec());
    println!("{:#b}, {:#b}", b, lower_b);
    assert_eq!(b, 0b11);
    assert_eq!(lower_b, 0b111);
}

pub fn demodulate_half_byte(stft: &mut STFT<f64>, fs: Vec<f64>) -> u8 {
    let result = stft_result(stft, &fs);
    let freq_col = result[result.len() / 2].to_owned();
    let freqs = detect_main_freqs(&freq_col);
    decode_by_given_freq_pattern(&CARRIER_FREQS, &freqs)
}

fn detect_main_freqs(freq_col: &[f64]) -> Vec<f64> {
    let mut freq_col_idx: Vec<(f64, usize)> = freq_col.to_owned().into_iter().zip(0..).collect();
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
    freqs
}

fn decode_by_given_freq_pattern(freq_pattern: &[f64], freqs: &[f64]) -> u8 {
    let mut byte_result = 0_u8;
    for freq in freqs {
        let idx = freq_pattern
            .binary_search_by(|probe| probe.partial_cmp(freq).unwrap())
            .unwrap();
        byte_result |= 1 << idx;
    }
    byte_result
}

pub fn prepend_preamble(signal: &[f64]) -> Vec<f64> {
    let a = PREAMBLE_SIGNALS.lock().unwrap()[0].clone();
    let b = PREAMBLE_SIGNALS.lock().unwrap()[1].clone();
    let mut s = [a, b].concat().repeat(2);
    s.extend_from_slice(signal);
    s
}

/// Preamble detection
#[derive(Debug, Clone, Copy)]
pub enum Preamble {
    NoPreamble,
    Detected {
        ending_position: usize,
        signal_bit: u8,
        votes: u8,
    },
}

pub fn detect_preamble(signal: &[f64]) -> Preamble {
    let mut stft = STFT::new(ruststft::WindowType::Hanning, 256, 128);
    let mut ending_position = 0;
    let mut zero_vote = 0;
    let mut one_vote = 0;
    let freq_cols = stft_result(&mut stft, signal);
    'outer: for col in freq_cols {
        let main_freqs = detect_main_freqs(&col);
        info!("freq: {:?}", main_freqs[0]);
        ending_position += stft.output_size();
        for main_freq in main_freqs {
            if (main_freq - PREAMBLE_FREQS[0]).abs() < 1e-1 {
                if one_vote != 0 {
                    ending_position -= stft.output_size();
                    break 'outer;
                }
                zero_vote += 1;
                break;
            } else if (main_freq - PREAMBLE_FREQS[1]).abs() < 1e-1 {
                if zero_vote != 0 {
                    ending_position -= stft.output_size();
                    break 'outer;
                }
                one_vote += 1;
                break;
            }
        }
    }
    match (zero_vote, one_vote) {
        (0, 0) => Preamble::NoPreamble,
        (x, y) => Preamble::Detected {
            ending_position,
            signal_bit: if x > y { 0 } else { 1 },
            votes: if x > y { zero_vote } else { one_vote },
        },
    }
}

#[test]
fn test_preamble() {
    let mut v = Vec::new();
    v = prepend_preamble(&v);
    println!("{:?}", detect_preamble(&v));
    output_wav(&v, "preamble.wav");
    let mut reader = hound::WavReader::open("preamble.wav").unwrap();
    let samples: Vec<f64> = reader.samples::<f32>().map(|f| f.unwrap() as f64).collect();
    println!("{:?}", detect_preamble(&v));
}

#[test]
fn test_preamble_zero() {
    let mut v = Vec::new();
    v.extend(PREAMBLE_SIGNALS.lock().unwrap()[0].repeat(100));
    output_wav(&v, "always0.wav");
}

#[test]
fn test_output_freqs() {
    use ruststft::STFT;
    let test_signal = SIGNALS.lock().unwrap()[0].clone();
    let mut sfft: STFT<f64> = STFT::new(ruststft::WindowType::Hanning, 256, 128);
    sfft.append_samples(&test_signal);
    let mut result = repeat(0.0).take(sfft.output_size()).collect::<Vec<f64>>();
    sfft.compute_column(&mut result);
    println!("{:?}, {}", sfft.freqs(44100.0), sfft.freqs(44100.0).len());
}
