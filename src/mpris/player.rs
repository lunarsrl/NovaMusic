// SPDX-License-Identifier: GPL-2.0-or-later

use crate::mpris::MPRISRootInterface;
use std::collections::HashMap;
use zbus::interface;

enum MdatValue {
    Uint(u32),
    Sint(i32),
    Float(f32),
    String(String),
}
struct MPRISPlayer {
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
}

#[interface(name = "org.mpris.MediaPlayer2.NovaMusic")]
impl MPRISRootInterface {}

impl MPRISPlayer {}
