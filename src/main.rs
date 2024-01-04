use acousticdi::{
    recorder::{run_record, Recorder},
    PREAMBLE, SIGNAL_TIME, SAMPLE_RATE, physics::demodulate_bit,
};
use tracing::info;

fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    info!("Hello, world!");
    let mut recorder = Recorder::new();
    let _stream = run_record(recorder.clone_handle()).unwrap();
    loop {
        let v: f32 = recorder.take_owned(1000).iter().sum();
        if (v - PREAMBLE * 1000.0).abs() > 1.0 {
            continue;
        }
        info!("detected signal preamble, verifying {}", v);
        if verified_signal(&mut recorder) {
            info!("Preamble valid, start decode!");
            break;
        }
        info!("sorry, I can not recognize that signal..");
    }
    let result = demodulate(&mut recorder);
    println!("Result:\n\t{}", result);
}

fn verified_signal(recorder: &mut Recorder) -> bool {
    let sound: f32 = recorder.take_owned(4410).iter().sum();
    sound > 4410.0 * PREAMBLE * 0.9
}

fn demodulate(recorder: &mut Recorder) -> String {
    loop {
        let signal = recorder.take_owned((SIGNAL_TIME * SAMPLE_RATE) as usize);
        print!("{}", demodulate_bit(signal));
    }
}
