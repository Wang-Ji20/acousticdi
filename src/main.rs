use acousticdi::{
    recorder::{run_record, Recorder},
    transmission::Receiver,
};
use tracing::info;

fn main() {
    let _ = tracing_subscriber::fmt::try_init();
    info!("Hello, world!");
    let mut recorder = Recorder::new();
    let _stream = run_record(recorder.clone_handle()).unwrap();

    let mut receiver = Receiver::new(Box::new(recorder));
    receiver.run();
}
