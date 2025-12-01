// SPDX-License-Identifier: GPL-2.0-or-later

use std::collections::HashMap;
use std::sync::Arc;
use zbus::interface;

enum MdatValue {
    Uint(u32),
    Sint(i32),
    Float(f32),
    String(String),
}

pub struct MPRISPlayer {
    playback_status: String,
    loop_stats: String,
    rate: f32,
    shuffle: bool,
    metadata: HashMap<String, MdatValue>,
    volume: f32,
    /// in microseconds
    position: u32,
    minimum_rate: f32,
    maximum_rate: f32,
    can_go_next: bool,
    can_go_previous: bool,
    can_play: bool,
    can_pause: bool,
    can_seek: bool,
    can_control: bool,
    pub event: Arc<event_listener::Event>,
}

#[interface(name = "org.mpris.MediaPlayer2.NovaMusic.Player")]
impl MPRISPlayer {
    #[zbus(property)]
    async fn playback_status(&self) -> &str {
        &self.playback_status
    }
    #[zbus(property)]
    async fn loop_state(&self) -> &str {
        &self.loop_stats
    }

    #[zbus(property)]
    async fn rate(&self) -> f32 {
        self.rate
    }

    #[zbus(property)]
    async fn shuffle(&self) -> bool {
        self.shuffle
    }

    #[zbus(property)]
    async fn position(&self) -> u32 {
        self.position
    }

    #[zbus(property)]
    async fn minimum_rate(&self) -> f32 {
        self.minimum_rate
    }

    #[zbus(property)]
    async fn maximum_rate(&self) -> f32 {
        self.maximum_rate
    }

    #[zbus(property)]
    async fn volume(&self) -> f32 {
        self.volume
    }

    #[zbus(property)]
    async fn can_go_next(&self) -> bool {
        self.can_go_next
    }

    #[zbus(property)]
    async fn can_go_previous(&self) -> bool {
        self.can_go_previous
    }

    #[zbus(property)]
    async fn can_play(&self) -> bool {
        self.can_play
    }
    #[zbus(property)]
    async fn can_seek(&self) -> bool {
        self.can_seek
    }
    #[zbus(property)]
    async fn can_control(&self) -> bool {
        self.can_control
    }

    #[zbus(property)]
    async fn can_pause(&self) -> bool {
        self.can_pause
    }

    async fn pause(&self) {
        self.event.notify(1);
    }

    async fn play(&self) {
        self.event.notify(1);
    }
}

impl MPRISPlayer {
    pub fn new() -> MPRISPlayer {
        let mut metadata = HashMap::new();
        metadata.insert(
            String::from("title"),
            MdatValue::String(String::from("More Track")),
        );
        MPRISPlayer {
            playback_status: "".to_string(),
            loop_stats: "".to_string(),
            rate: 0.0,
            shuffle: false,
            metadata,
            volume: 0.0,
            position: 0,
            minimum_rate: 0.0,
            maximum_rate: 0.0,
            can_go_next: false,
            can_go_previous: false,
            can_play: false,
            can_pause: true,
            can_seek: false,
            can_control: false,
            event: Arc::from(event_listener::Event::new()),
        }
    }
}
