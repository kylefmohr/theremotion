#![warn(clippy::all, rust_2018_idioms)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod dsp;
mod leap;

#[allow(clippy::all)]
mod faust {
    include!(concat!(env!("OUT_DIR"), "/dsp.rs"));
}

use std::sync::{Arc, Mutex};

use cpal::traits::StreamTrait;
use faust_state::DspHandle;

fn main() {
    // Log to stdout (if you run with `RUST_LOG=debug`).
    tracing_subscriber::fmt::init();

    // Init DSP
    let (dsp, state) = DspHandle::<faust::Dumbosc>::new();
    let dsp = Box::new(dsp);
    let state = Arc::new(Mutex::new(state));

    // Init sound output
    let stream = dsp::run_dsp(dsp);
    stream.play().expect("Failed to play silence");

    // Init leap thread
    let _leap_worker = leap::start_leap_worker(state.clone());

    // Start UI
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "eframe template",
        native_options,
        Box::new(move |cc| Box::new(app::Leapotron::new(cc, state))),
    );
}
