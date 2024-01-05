//! # Transmission layer
//!
//! Transmission layer cuts down the recorded audio to small pieces which then sent to the
//! physics layer. It utilizes the decoded information to continuously cut down input audio
//! to pieces.
//!

use tracing::info;

use crate::{
    physics::{detect_preamble, Preamble, PREAMBLE_FREQS},
    recorder::Recorder,
};

pub const SAMPLE_RATE: f64 = 44100.0;

pub const SIGNAL_TIME: f64 = 0.1;

pub const SAMPLE_NUMBER: usize = (SAMPLE_RATE * SIGNAL_TIME) as usize;
pub const PROBE_SAMPLE_NUMBER: usize = 256;

pub trait SampleReader {
    fn take_samples(&mut self, start: usize, end: usize) -> Vec<f64>;
}

/// This is essentially a Turing machine
pub struct Receiver {
    reader: Box<dyn SampleReader>,
    processed_samples: usize,
}

impl Receiver {
    pub fn new(recorder: Box<dyn SampleReader>) -> Receiver {
        Receiver {
            reader: recorder,
            processed_samples: 0,
        }
    }

    pub fn run(&mut self) {
        loop {
            if self.detect_preambles(0) {
                if self.verify_preamble() {
                    panic!("get it");
                }
            }
        }
    }

    fn take_samples(&mut self) -> Vec<f64> {
        self.reader.take_samples(
            self.processed_samples,
            self.processed_samples + SAMPLE_NUMBER,
        )
    }

    fn take_probe_samples(&mut self) -> Vec<f64> {
        self.reader.take_samples(
            self.processed_samples,
            self.processed_samples + PROBE_SAMPLE_NUMBER,
        )
    }

    fn verify_preamble(&mut self) -> bool {
        for i in [1, 0, 1] {
            if !self.detect_preambles(i) {
                return false;
            }
        }
        info!("verified data pack");
        true
    }

    /// probe and detect *bit*. wait until see bit. consume all bit and calculate vote
    fn detect_preambles(&mut self, bit: u8) -> bool {
        loop {
            let samples = self.take_probe_samples();
            match detect_preamble(&samples) {
                crate::physics::Preamble::NoPreamble => {
                    self.processed_samples += PROBE_SAMPLE_NUMBER;
                    continue;
                }
                crate::physics::Preamble::Detected {
                    ending_position,
                    signal_bit,
                    votes,
                } => {
                    self.processed_samples += ending_position;
                    // 0 -> 0
                    if signal_bit != bit {
                        continue;
                    }
                    // 0 -> 1
                    info!("probed preamble {}", signal_bit);
                    let mut samples = self.take_probe_samples();
                    let mut cumulated_pos_votes = 0;
                    let mut cumulated_neg_votes = 0;
                    let mut cumulated_spaces = 0;
                    loop {
                        match detect_preamble(&samples) {
                            Preamble::Detected {
                                ending_position,
                                signal_bit,
                                votes,
                            } => {
                                info!("collecting preamble {}", signal_bit);
                                if signal_bit != bit {
                                    cumulated_neg_votes += 1;
                                    if cumulated_neg_votes > 3 {
                                        info!("not correct, fallback");
                                        break;
                                    }
                                }
                                cumulated_pos_votes += votes;
                                self.processed_samples += ending_position;
                                samples = self.take_probe_samples();
                            }
                            Preamble::NoPreamble => {
                                info!("gotten some noises");
                                self.processed_samples += PROBE_SAMPLE_NUMBER;
                                cumulated_spaces += 1;
                                if cumulated_spaces > 30 {
                                    break;
                                } else {
                                    continue;
                                }
                            }
                        }
                    }
                    if cumulated_pos_votes > 20 {
                        info!(
                            "because get {} votes, {} is verified",
                            cumulated_pos_votes, bit
                        );
                        return true;
                    }
                }
            }
        }
    }

    fn demodulate_data(&mut self) -> Vec<u8> {
        // self.consume_lagging_preambles();
        todo!();
    }
}

#[cfg(test)]
mod tests {
    use std::iter::repeat;

    use tracing::info;

    use crate::physics::prepend_preamble;

    use super::*;

    struct MockSampleReader(pub Vec<f64>);

    impl SampleReader for MockSampleReader {
        fn take_samples(&mut self, start: usize, end: usize) -> Vec<f64> {
            info!("taking sample from {} to {}", start, end);
            assert!(start < end && end <= self.0.len());
            self.0[start..end].to_owned()
        }
    }

    #[test]
    fn test_read_preamble() {
        let _ = tracing_subscriber::fmt::try_init();
        let mut v = repeat(0.5_f64).take(10000).collect::<Vec<f64>>();
        v = prepend_preamble(&mut v);
        let mut receiver = Receiver::new(Box::new(MockSampleReader(v)));
        receiver.run();
    }

    #[test]
    fn test_read_zeros() {
        let _ = tracing_subscriber::fmt::try_init();
        let mut reader = hound::WavReader::open("recorder.wav").unwrap();
        let samples: Vec<f64> = reader.samples::<f32>().map(|f| f.unwrap() as f64).collect();
        println!("{}", samples.len());
        let mut receiver = Receiver::new(Box::new(MockSampleReader(samples)));
        receiver.run();
    }
}
