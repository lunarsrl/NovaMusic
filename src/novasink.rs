use std::ops::Deref;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::time::Duration;
use rodio::mixer::Mixer;
use rodio::Source;
use rodio::source::{Amplify, Pausable, TrackPosition};

struct Controls {
    pause: AtomicBool,
    volume: Mutex<f32>,
    stopped: AtomicBool,
    speed: Mutex<f32>,
    to_clear: Mutex<u32>,
    // seek: Mutex<Option<SeekOrder>>,
    position: Mutex<Duration>,
}

/// Rodio sink but without using rodio queue so that Nova Music can handle scheduling things
pub struct NovaSink {
    source: Option<Box<dyn Source>>,
    controls: Arc<Controls>,
}

impl NovaSink {
    pub fn new() -> (NovaSink) {
        NovaSink {
            source: None,
            controls: Arc::new(Controls {
                pause: AtomicBool::new(false),
                volume: Mutex::new(1.0),
                stopped: AtomicBool::new(false),
                speed: Mutex::new(1.0),
                to_clear: Mutex::new(0),
                position: Mutex::new(Duration::ZERO),
            }),
        }

    }

    pub fn connect_new(mixer: &Mixer) -> NovaSink {
        let (sink, source) = NovaSink::new();
        mixer.add(source);
        sink
    }

    pub fn append<S>(&self, source: S)
     where S: Source + Send + 'static,
     {
         let controls = self.controls.clone();
         source
             .track_position()
             .pausable(false)
             .amplify(1.0)
             .periodic_access(Duration::from_millis(2), |control| {
                 *controls.position.lock().unwrap() = control.inner().inner().get_pos();
                 control.inner_mut().set_paused(self.controls.pause.load(Ordering::Relaxed));
             });

         let (tx, rx): (Sender<()>, Receiver<()>) = std::sync::mpsc::channel();

     }

    pub fn pause(&self) {
        self.controls.pause.store(true, Ordering::Relaxed);
    }

    pub fn play(&self) {
        self.controls.pause.store(false, Ordering::Relaxed);
    }
}