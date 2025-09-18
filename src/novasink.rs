use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

struct Controls {
    pause: AtomicBool,
    volume: Mutex<f32>,
    stopped: AtomicBool,
    speed: Mutex<f32>,
    to_clear: Mutex<u32>,
    position: Mutex<Duration>,
}

pub struct NovaSink {

}

impl NovaSink {

    pub fn new() -> NovaSink {
        NovaSink {}
    }
}
