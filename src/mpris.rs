// SPDX-License-Identifier: GPL-2.0-or-later

pub mod player;

use zbus::interface;
pub struct MPRISRootInterface {
    can_quit: bool,
    fullscreen: bool,
    can_set_fullscreen: bool,
    can_raise: bool,
    has_track_list: bool,
    identity: String,
    desktop_entry: String,
    supported_uri_schemes: String,
    supported_mime_types: String,
    pub event: event_listener::Event,
}

#[interface(name = "org.mpris.MediaPlayer2.NovaMusic")]
impl MPRISRootInterface {
    #[zbus(property)]
    async fn supported_mime_types(&self) -> &str {
        &self.supported_mime_types
    }
    #[zbus(property)]
    async fn supported_uri_schemes(&self) -> &str {
        &self.supported_uri_schemes
    }
    #[zbus(property)]
    async fn desktop_entry(&self) -> &str {
        &self.desktop_entry
    }

    #[zbus(property)]
    async fn has_track_list(&self) -> bool {
        self.has_track_list
    }
    #[zbus(property)]
    async fn can_set_fullscreen(&self) -> bool {
        self.can_set_fullscreen
    }

    #[zbus(property)]
    async fn identity(&self) -> &str {
        &self.identity
    }

    #[zbus(property)]
    async fn can_quit(&self) -> bool {
        self.can_quit
    }

    #[zbus(property)]
    async fn can_raise(&self) -> bool {
        self.can_raise
    }

    #[zbus(property)]
    async fn fullscreen(&self) -> bool {
        self.fullscreen
    }
}

impl MPRISRootInterface {
    pub fn new() -> MPRISRootInterface {
        MPRISRootInterface {
            can_quit: true,
            fullscreen: false,
            can_set_fullscreen: false,
            can_raise: false,
            has_track_list: true,
            identity: "Nova Music".to_string(),
            desktop_entry: "NovaMusic".to_string(),
            supported_uri_schemes: "".to_string(),
            supported_mime_types: "".to_string(),
            event: event_listener::Event::new(),
        }
    }
}
