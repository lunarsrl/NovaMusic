// SPDX-License-Identifier: GPL-2.0-or-later

use std::fmt::Display;
use strum_macros::EnumString;
use symphonia::core::meta::StandardTagKey;
use crate::log::setup_logger;

mod app;
mod config;
mod i18n;
mod log;
mod database;

fn main() -> cosmic::iced::Result {
    //start logging
    let logger = setup_logger();
    // Get the system's preferred languages.
    let requested_languages = i18n_embed::DesktopLanguageRequester::requested_languages();

    // Enable localizations to be applied.
    i18n::init(&requested_languages);

    // Settings for configuring the application window and iced runtime.
    let settings = cosmic::app::Settings::default().size_limits(
        cosmic::iced::Limits::NONE
            .min_width(360.0)
            .min_height(180.0),
    );

    // Starts the application's event loop with `()` as the application's flags.
    cosmic::app::run::<app::AppModel>(settings, ())
}


struct StandardTagKeyExt(StandardTagKey);


impl ToString for StandardTagKeyExt {
    fn to_string(&self) -> String {
        match self.0 {
            StandardTagKey::AcoustidFingerprint => {
                "AcoustID Fingerprint".to_string()
            }
            StandardTagKey::AcoustidId => {
                "AcoustID".to_string()
            }
            StandardTagKey::Album => {
                "Album".to_string()
            }
            StandardTagKey::AlbumArtist => {
                "Album Artist".to_string()
            }
            StandardTagKey::Arranger => {
                "Arranger".to_string()
            }
            StandardTagKey::Artist => {
                "Artist".to_string()
            }
            StandardTagKey::Bpm => {
                "BPM".to_string()
            }
            StandardTagKey::Comment => {
                "Comment".to_string()
            }
            StandardTagKey::Compilation => {
                "Compilation".to_string()
            }
            StandardTagKey::Composer => {
                "Composer".to_string()
            }
            StandardTagKey::Conductor => {
                "Conductor".to_string()
            }
            StandardTagKey::ContentGroup => {
                "Content Group".to_string()
            }
            StandardTagKey::Copyright => {
                "Copyright".to_string()
            }
            StandardTagKey::Date => {
                "Date".to_string()
            }
            StandardTagKey::Description => {
                "Description".to_string()
            }
            StandardTagKey::DiscNumber => {
                "Disc Number".to_string()
            }
            StandardTagKey::DiscSubtitle => {
                "Disc Subtitle".to_string()
            }
            StandardTagKey::DiscTotal => {
                "Disc Total".to_string()
            }
            StandardTagKey::EncodedBy => {
                "Encoded By".to_string()
            }
            StandardTagKey::Encoder => {
                "Encoder".to_string()
            }
            StandardTagKey::EncoderSettings => {
                "Encoder Settings".to_string()
            }
            StandardTagKey::EncodingDate => {
                "Encoding Date".to_string()
            }
            StandardTagKey::Engineer => {
                "Engineer".to_string()
            }
            StandardTagKey::Ensemble => {
                "Ensemble".to_string()
            }
            StandardTagKey::Genre => {
                "Genre".to_string()
            }
            StandardTagKey::IdentAsin => {
                "ASIN".to_string()
            }
            StandardTagKey::IdentBarcode => {
                "Barcode".to_string()
            }
            StandardTagKey::IdentCatalogNumber => {
                "Catalog Number".to_string()
            }
            StandardTagKey::IdentEanUpn => {
                "EAN/UPN".to_string()
            }
            StandardTagKey::IdentIsrc => {
                "ISRC".to_string()
            }
            StandardTagKey::IdentPn => {
                "PN".to_string()
            }
            StandardTagKey::IdentPodcast => {
                "Podcast".to_string()
            }
            StandardTagKey::IdentUpc => {
                "UPC".to_string()
            }
            StandardTagKey::Label => {
                "Label".to_string()
            }
            StandardTagKey::Language => {
                "Language".to_string()
            }
            StandardTagKey::License => {
                "License".to_string()
            }
            StandardTagKey::Lyricist => {
                "Lyricist".to_string()
            }
            StandardTagKey::Lyrics => {
                "Lyrics".to_string()
            }
            StandardTagKey::MediaFormat => {
                "Media Format".to_string()
            }
            StandardTagKey::MixDj => {
                "DJ Mix".to_string()
            }
            StandardTagKey::MixEngineer => {
                "Mix Engineer".to_string()
            }
            StandardTagKey::Mood => {
                "Mood".to_string()
            }
            StandardTagKey::MovementName => {
                "Movement Name".to_string()
            }
            StandardTagKey::MovementNumber => {
                "Movement Number".to_string()
            }
            StandardTagKey::MusicBrainzAlbumArtistId => {
                "MusicBrainz Album Artist ID".to_string()
            }
            StandardTagKey::MusicBrainzAlbumId => {
                "MusicBrainz Album ID".to_string()
            }
            StandardTagKey::MusicBrainzArtistId => {
                "MusicBrainz Artist ID".to_string()
            }
            StandardTagKey::MusicBrainzDiscId => {
                "MusicBrainz Disc ID".to_string()
            }
            StandardTagKey::MusicBrainzGenreId => {
                "MusicBrainz Genre ID".to_string()
            }
            StandardTagKey::MusicBrainzLabelId => {
                "MusicBrainz Label ID".to_string()
            }
            StandardTagKey::MusicBrainzOriginalAlbumId => {
                "MusicBrainz Original Album ID".to_string()
            }
            StandardTagKey::MusicBrainzOriginalArtistId => {
                "MusicBrainz Original Artist ID".to_string()
            }
            StandardTagKey::MusicBrainzRecordingId => {
                "MusicBrainz Recording ID".to_string()
            }
            StandardTagKey::MusicBrainzReleaseGroupId => {
                "MusicBrainz Release Group ID".to_string()
            }
            StandardTagKey::MusicBrainzReleaseStatus => {
                "MusicBrainz Release Status".to_string()
            }
            StandardTagKey::MusicBrainzReleaseTrackId => {
                "MusicBrainz Release Track ID".to_string()
            }
            StandardTagKey::MusicBrainzReleaseType => {
                "MusicBrainz Release Type".to_string()
            }
            StandardTagKey::MusicBrainzTrackId => {
                "MusicBrainz Track ID".to_string()
            }
            StandardTagKey::MusicBrainzWorkId => {
                "MusicBrainz Work ID".to_string()
            }
            StandardTagKey::Opus => {
                "Opus".to_string()
            }
            StandardTagKey::OriginalAlbum => {
                "Original Album".to_string()
            }
            StandardTagKey::OriginalArtist => {
                "Original Artist".to_string()
            }
            StandardTagKey::OriginalDate => {
                "Original Date".to_string()
            }
            StandardTagKey::OriginalFile => {
                "Original File".to_string()
            }
            StandardTagKey::OriginalWriter => {
                "Original Writer".to_string()
            }
            StandardTagKey::Owner => {
                "Owner".to_string()
            }
            StandardTagKey::Part => {
                "Part".to_string()
            }
            StandardTagKey::PartTotal => {
                "Part Total".to_string()
            }
            StandardTagKey::Performer => {
                "Performer".to_string()
            }
            StandardTagKey::Podcast => {
                "Podcast".to_string()
            }
            StandardTagKey::PodcastCategory => {
                "Podcast Category".to_string()
            }
            StandardTagKey::PodcastDescription => {
                "Podcast Description".to_string()
            }
            StandardTagKey::PodcastKeywords => {
                "Podcast Keywords".to_string()
            }
            StandardTagKey::Producer => {
                "Producer".to_string()
            }
            StandardTagKey::PurchaseDate => {
                "Purchase Date".to_string()
            }
            StandardTagKey::Rating => {
                "Rating".to_string()
            }
            StandardTagKey::ReleaseCountry => {
                "Release Country".to_string()
            }
            StandardTagKey::ReleaseDate => {
                "Release Date".to_string()
            }
            StandardTagKey::Remixer => {
                "Remixer".to_string()
            }
            StandardTagKey::ReplayGainAlbumGain => {
                "Replay Gain Album Gain".to_string()
            }
            StandardTagKey::ReplayGainAlbumPeak => {
                "Replay Gain Album Peak".to_string()
            }
            StandardTagKey::ReplayGainTrackGain => {
                "Replay Gain Track Gain".to_string()
            }
            StandardTagKey::ReplayGainTrackPeak => {
                "Replay Gain Track Peak".to_string()
            }
            StandardTagKey::Script => {
                "Script".to_string()
            }
            StandardTagKey::SortAlbum => {
                "Sort Album".to_string()
            }
            StandardTagKey::SortAlbumArtist => {
                "Sort Album Artist".to_string()
            }
            StandardTagKey::SortArtist => {
                "Sort Artist".to_string()
            }
            StandardTagKey::SortComposer => {
                "Sort Composer".to_string()
            }
            StandardTagKey::SortTrackTitle => {
                "Sort Track Title".to_string()
            }
            StandardTagKey::TaggingDate => {
                "Tagging Date".to_string()
            }
            StandardTagKey::TrackNumber => {
                "Track Number".to_string()
            }
            StandardTagKey::TrackSubtitle => {
                "Track Subtitle".to_string()
            }
            StandardTagKey::TrackTitle => {
                "Track Title".to_string()
            }
            StandardTagKey::TrackTotal => {
                "Track Total".to_string()
            }
            StandardTagKey::TvEpisode => {
                "TV Episode".to_string()
            }
            StandardTagKey::TvEpisodeTitle => {
                "TV Episode Title".to_string()
            }
            StandardTagKey::TvNetwork => {
                "TV Network".to_string()
            }
            StandardTagKey::TvSeason => {
                "TV Season".to_string()
            }
            StandardTagKey::TvShowTitle => {
                "TV Show Title".to_string()
            }
            StandardTagKey::Url => {
                "URL".to_string()
            }
            StandardTagKey::UrlArtist => {
                "URL Artist".to_string()
            }
            StandardTagKey::UrlCopyright => {
                "URL Copyright".to_string()
            }
            StandardTagKey::UrlInternetRadio => {
                "URL Internet Radio".to_string()
            }
            StandardTagKey::UrlLabel => {
                "URL Label".to_string()
            }
            StandardTagKey::UrlOfficial => {
                "URL Official".to_string()
            }
            StandardTagKey::UrlPayment => {
                "URL Payment".to_string()
            }
            StandardTagKey::UrlPodcast => {
                "URL Podcast".to_string()
            }
            StandardTagKey::UrlPurchase => {
                "URL Purchase".to_string()
            }
            StandardTagKey::UrlSource => {
                "URL Source".to_string()
            }
            StandardTagKey::Version => {
                "Version".to_string()
            }
            StandardTagKey::Writer => {
                "Writer".to_string()
            }
        }
    }
}

