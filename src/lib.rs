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
            .map(|x| &x.data)
            .fold(Vec::new(), |mut acc, f| {
                acc.extend_from_slice(f);
                acc
            })
    }

    fn seal_one(self: &Self) -> Vec<u8> {
        let mut packet = Vec::new();
        packet.extend_from_slice(&self.order.to_le_bytes());
        packet.extend_from_slice(&self.data.len().to_le_bytes());
        packet.extend_from_slice(&self.data);
        packet
    }

    pub fn seal(s: &[Packet]) -> Vec<Vec<u8>> {
        s.into_iter().map(Self::seal_one).collect()
    }

    fn unseal_one(v: &[u8]) -> Self {
        let order = usize::from_le_bytes(v[0..8].try_into().unwrap());
        let len = usize::from_le_bytes(v[8..16].try_into().unwrap());
        let data = v[16..16 + len].to_vec();
        Self { order, data }
    }

    pub fn unseal(v: &[Vec<u8>]) -> Vec<Packet> {
        v.into_iter().map(|x| Self::unseal_one(x)).collect()
    }
}

#[test]
fn pack_unpack_test() {
    let data = "WHAT is truth? said jesting Pilate, and would not stay for an answer. Certainly there be, that delight in giddiness, and count it a bondage to fix a belief; affecting free-will in thinking, as well as in acting. And though the sects of philosophers of that kind be gone, yet there remain certain discoursing wits, which are of the same veins, though there be not so much blood in them, as was in those of the ancients. But it is not only the difficulty and labor, which men take in finding out of truth, nor again, that when it is found, it imposeth upon men's thoughts, that doth bring lies in favor; but a natural, though corrupt love, of the lie itself. One of the later school of the Grecians, examineth the matter, and is at a stand, to think what should be in it, that men should love lies; where neither they make for pleasure, as with poets, nor for advantage, as with the merchant; but for the lie's sake. But I cannot tell; this same truth, is a naked, and open day-light, that doth not show the masks, and mummeries, and triumphs, of the world, half so stately and daintily as candle-lights. Truth may perhaps come to the price of a pearl, that showeth best by day; but it will not rise to the price of a diamond, or carbuncle, that showeth best in varied lights. A mixture of a lie doth ever add pleasure. Doth any man doubt, that if there were taken out of men's minds, vain opinions, flattering hopes, false valuations, imaginations as one would, and the like, but it would leave the minds, of a number of men, poor shrunken things, full of melancholy and indisposition, and unpleasing to themselves?";
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

use std::sync::Mutex;

use dasp::{signal, Signal};

/// Generate sound wave to carry the information.
/// For first version, I will just use BPSK modulation.
pub fn modulate(segments: Vec<Vec<u8>>) -> Vec<f64> {
    segments
        .into_iter()
        .map(modulate_vector)
        .flatten()
        .collect()
}

fn modulate_vector(p: Vec<u8>) -> Vec<f64> {
    p.into_iter().map(modulate_byte).flatten().collect()
}

fn modulate_byte(b: u8) -> Vec<f64> {
    (0..8)
        .map(|i| modulate_bit(b & (1 << i) >> i))
        .flatten()
        .collect()
}

use once_cell::sync::Lazy;

static ZERO_SIGNAL: Lazy<Mutex<Vec<f64>>> = Lazy::new(|| {
    Mutex::new(
        signal::rate(44100.0)
            .const_hz(440.0)
            .phase()
            .sine()
            .take(4400)
            .collect::<Vec<f64>>(),
    )
});

fn modulate_bit(b: u8) -> Vec<f64> {
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

#[test]
fn test_modulate() {
    let data = "hello world";
    let modulated = modulate(Packet::seal(&Packet::new_packets(&encode(data))));
    assert_eq!(modulated.len(), 4400 * 8 * (16 + 11));
}

use hound;
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
    let data = "hello world";
    let modulated = modulate(Packet::seal(&Packet::new_packets(&encode(data))));
    output_wav(&modulated, "test.wav");
}

use hound::WavReader;
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

