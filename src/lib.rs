pub mod recorder;

const TEST_DATA: &str = "WHAT is truth? said jesting Pilate and would not stay for an answer. Certainly there be that delight";

/// To transmit data, we need to encode them to a byte array first.
pub fn encode(data: &str) -> Vec<u8> {
    data.as_bytes().to_vec()
}

/// To receive data, we need to decode them from a byte array.
pub fn decode(data: &[u8]) -> String {
    String::from_utf8_lossy(data).to_string()
}

#[test]
fn test_encode_decode() {
    let data = "hello world";
    let encoded = encode(data);
    let decoded = decode(&encoded);
    assert_eq!(data, decoded);
}

#[derive(Clone, Debug)]
pub struct Packet {
    pub order: usize,
    pub data: Vec<u8>,
}

impl From<(usize, &[u8])> for Packet {
    fn from((order, data): (usize, &[u8])) -> Self {
        Self {
            order,
            data: data.to_vec(),
        }
    }
}

impl Packet {
    /// Longer data are splitted to multiple packets, here is the threshold(in bytes)
    const MAX_PACKET_SIZE: usize = 128;

    /// split a long long data to packets
    pub fn new_packets(v: &[u8]) -> Vec<Packet> {
        v.chunks(Self::MAX_PACKET_SIZE)
            .enumerate()
            .map(Packet::from)
            .collect()
    }

    pub fn unpack(vp: &[Packet]) -> Vec<u8> {
        let mut sorted_vp = vp.to_vec();
        sorted_vp.sort_by_key(|x| x.order);
        sorted_vp
            .iter()
            .flat_map(|x| x.data.clone())
            .collect::<Vec<u8>>()
    }

    fn seal_one(&self) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&self.order.to_le_bytes());
        packet.extend_from_slice(&self.data.len().to_le_bytes());
        packet.extend_from_slice(&self.data);
        packet
    }

    pub fn seal(s: &[Packet]) -> Vec<Vec<u8>> {
        s.iter().map(Self::seal_one).collect()
    }

    fn unseal_one(v: &[u8]) -> Self {
        let order = usize::from_le_bytes(v[0..8].try_into().unwrap());
        let len = usize::from_le_bytes(v[8..16].try_into().unwrap());
        let data = v[16..16 + len].to_vec();
        Self { order, data }
    }

    pub fn unseal(v: &[Vec<u8>]) -> Vec<Packet> {
        v.iter().map(|x| Self::unseal_one(x)).collect()
    }
}

#[test]
fn pack_unpack_test() {
    let data = "WHAT is truth? said jesting Pilate, and would not stay for an answer. Certainly there be, that delight";
    let packets = Packet::new_packets(&encode(data));
    let unpacked = Packet::unpack(&packets);
    assert_eq!(data, decode(&unpacked));
}

#[test]
fn pack_unseal_test() {
    let data = "hello world";
    let packets = Packet::new_packets(&encode(data));
    let sealed = Packet::seal(&packets);
    let unsealed = Packet::unseal(&sealed);
    let unpacked = Packet::unpack(&unsealed);
    assert_eq!(data, decode(&unpacked));
}

use std::{fs::File, io::BufWriter};
pub mod physics;
pub mod transmission;

/// Generate sound wave to carry the information.
/// For first version, I will just use BPSK modulation.
pub fn modulate(segments: Vec<Vec<u8>>) -> Vec<f64> {
    segments.into_iter().flat_map(modulate_vector).collect()
}

fn modulate_vector(p: Vec<u8>) -> Vec<f64> {
    p.into_iter().flat_map(modulate_byte).collect()
}

fn modulate_byte(b: u8) -> Vec<f64> {
    (0..8)
        .flat_map(|i| modulate_half_byte(b & (1 << i) >> i))
        .collect()
}

#[test]
fn test_modulate() {
    let data = "hello world";
    let modulated = modulate(Packet::seal(&Packet::new_packets(&encode(data))));
    assert_eq!(modulated.len(), 4400 * 8 * (16 + 11));
}

/// output the sound wave to a wav file
pub fn output_wav(modulated: &[f64], filename: &str) {
    let spec = hound::WavSpec {
        channels: 1,
        sample_rate: 44100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    let mut writer = hound::WavWriter::create(filename, spec).unwrap();
    for sample in modulated {
        writer.write_sample(*sample as f32).unwrap();
    }
    writer.finalize().unwrap();
}

#[test]
fn test_output_wav() {
    let data = TEST_DATA;
    let modulated = modulate(Packet::seal(&Packet::new_packets(&encode(data))));
    output_wav(&modulated, "test.wav");
}

use hound::WavReader;
use physics::modulate_half_byte;
/// read the sound wave from a wav file
pub fn input_wav(filename: &str) -> Vec<f64> {
    let mut reader = WavReader::open(filename).unwrap();
    let samples: Vec<f64> = reader.samples::<f32>().map(|x| x.unwrap() as f64).collect();
    samples
}

#[test]
fn test_input_wav() {
    let data = "hello world";
    let modulated = modulate(Packet::seal(&Packet::new_packets(&encode(data))));
    output_wav(&modulated, "test.wav");
    let input = input_wav("test.wav");
    assert_eq!(modulated.len(), input.len());
}
