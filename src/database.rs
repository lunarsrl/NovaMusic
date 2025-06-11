use rusqlite::OptionalExtension;
use std::any::Any;
use std::fmt::format;
use std::path::PathBuf;
use futures_util::future::err;
use symphonia::core::meta::{StandardTagKey, Tag, Value};

struct Artist {
    id: u64,
    name: Option<String>,
}

struct Album {
    id: u32,
    name: String,
    artist_id: u64,
    num_of_discs: u64,
    num_of_tracks: u64,
    album_cover: Option<Vec<u8>>,
}

struct Track {
    id: u64,
    artist_id: Option<u64>,
    name: Option<String>,
    path: PathBuf,

}

struct AlbumTracks {
    id: u64,
    album_id: u32,
    track_id: u32,
    track_number: u64,
    disc_number: u64,
}

struct Playlist {
    id: u64,
    name: String,
}

struct PlaylistTracks {
    id: u64,
    playlist_id: u64,
    track_id: u64,
}

pub fn create_database() {
    let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();

    conn.execute_batch(
        "
        DROP TABLE IF EXISTS temp_album;
        DROP TABLE IF EXISTS album;
        DROP TABLE IF EXISTS album_tracks;
        DROP TABLE IF EXISTS artists;
        DROP TABLE IF EXISTS playlists;
        DROP TABLE IF EXISTS track;
    ",
    )
    .unwrap();

    conn.execute(
        "
    CREATE TABLE artists (
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE
    )",
        [],
    )
    .unwrap();

    conn.execute(
        "
    CREATE TABLE playlists (
        id INTEGER PRIMARY KEY,
        name TEXT
    )",
        [],
    )
    .unwrap();

    conn.execute(
        "
    CREATE TABLE album_tracks (
        id INTEGER PRIMARY KEY,
        album_id INTEGER,
        track_id INTEGER,
        track_number INTEGER,
        disc_number INTEGER,
        FOREIGN KEY(album_id) REFERENCES albums(id),
        FOREIGN KEY(track_id) REFERENCES tracks(id)
    )",
        [],
    )
    .unwrap();

    conn.execute(
        "
    CREATE TABLE track (
        id INTEGER PRIMARY KEY,
        name TEXT,
        path TEXT,
        artist_id INTEGER,
        FOREIGN KEY(artist_id) REFERENCES artist(id)
    )",
        [],
    )
    .unwrap();

    conn.execute(
        "
        CREATE TABLE album (
            id INTEGER PRIMARY KEY,
            name TEXT UNIQUE,
            artist_id INTEGER,
            disc_number INTEGER,
            track_number INTEGER,
            FOREIGN KEY(artist_id) REFERENCES artist(id)
        )",
        [],
    )
    .unwrap();
}
pub fn create_database_entry(metadata_tags: Vec<Tag>, filepath: &PathBuf) {
    let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();

    let mut track = Track {
        id: 0,
        artist_id: None,
        name: None,
        path: filepath.clone(),
    };

    let mut album = Album {
        id: 0,
        name: "".to_string(),
        artist_id: 0,
        num_of_discs: 0,
        num_of_tracks: 0,
        album_cover:  None,
    };

    let mut album_tracks = AlbumTracks {
        id: 0,
        album_id: 0,
        track_id: 0,
        disc_number: 0,
        track_number: 0,
    };
    
    let mut artist = Artist {
        id: 0,
        name: None,
    };

    for tag in metadata_tags {
        if let Some(key) = tag.std_key {
            match key {
                StandardTagKey::AcoustidFingerprint => {}
                StandardTagKey::AcoustidId => {}
                StandardTagKey::Album => match tag.value {
                    Value::String(name) => {
                        album.name = name;
                    }
                    _ => {
                        log::error!("Album name is not a string");
                    }
                },
                StandardTagKey::AlbumArtist => {
                    match tag.value {
                        Value::String(name) => {
                            match conn.execute(
                                "INSERT INTO artists (name) VALUES (?)",
                                [&name],
                            ) {
                                Ok(_) => {
                                    log::info!("Added artist {} to artists", name);
                                    album.artist_id = conn.last_insert_rowid() as u64;
                                }
                                Err(_) => {
                                    log::warn!("Artist: {} already created", name);
                                    
                                    album.artist_id = conn.query_row(
                                        "SELECT id FROM artists WHERE name = ?",
                                        &[&name],
                                        |row| {
                                            row.get::<usize, u32>(0)
                                        },
                                    ).unwrap() as u64;
                                    
                                    log::info!("ARTIST ID:  {}", album.artist_id);
                                }
                            }
                        }
                        _ => {}
                    }
                }
                StandardTagKey::Arranger => {}
                StandardTagKey::Artist => match tag.value {
                    Value::String(name) => {
                       artist.name = Some(name); 
                    }
                    _ => {
                        log::error!("Artist name is not a string");
                    }
                },
                StandardTagKey::Bpm => {}
                StandardTagKey::Comment => {}
                StandardTagKey::Compilation => {}
                StandardTagKey::Composer => {}
                StandardTagKey::Conductor => {}
                StandardTagKey::ContentGroup => {}
                StandardTagKey::Copyright => {}
                StandardTagKey::Date => {}
                StandardTagKey::Description => {}
                StandardTagKey::DiscNumber => match tag.value {
                    Value::String(val) => {
                        let mut final_val = val;

                        if final_val.contains("/") {
                            final_val = final_val.split("/").next().expect("Number").parse().unwrap();
                        }

                        album_tracks.disc_number = final_val.parse::<u64>().expect(format!("Invalid track number: {}", final_val).as_str());
                    }
                    _ => {
                        log::error!("Disc number is not a number");
                    }
                },
                StandardTagKey::DiscSubtitle => {}
                StandardTagKey::DiscTotal => match tag.value {
                    Value::String(val) => album.num_of_discs = val.parse::<u64>().unwrap(),
                    _ => {
                        log::error!("Disc number is not a number");
                    }
                },
                StandardTagKey::EncodedBy => {}
                StandardTagKey::Encoder => {}
                StandardTagKey::EncoderSettings => {}
                StandardTagKey::EncodingDate => {}
                StandardTagKey::Engineer => {}
                StandardTagKey::Ensemble => {}
                StandardTagKey::Genre => {}
                StandardTagKey::IdentAsin => {}
                StandardTagKey::IdentBarcode => {}
                StandardTagKey::IdentCatalogNumber => {}
                StandardTagKey::IdentEanUpn => {}
                StandardTagKey::IdentIsrc => {}
                StandardTagKey::IdentPn => {}
                StandardTagKey::IdentPodcast => {}
                StandardTagKey::IdentUpc => {}
                StandardTagKey::Label => {}
                StandardTagKey::Language => {}
                StandardTagKey::License => {}
                StandardTagKey::Lyricist => {}
                StandardTagKey::Lyrics => {}
                StandardTagKey::MediaFormat => {}
                StandardTagKey::MixDj => {}
                StandardTagKey::MixEngineer => {}
                StandardTagKey::Mood => {}
                StandardTagKey::MovementName => {}
                StandardTagKey::MovementNumber => {}
                StandardTagKey::MusicBrainzAlbumArtistId => {}
                StandardTagKey::MusicBrainzAlbumId => {}
                StandardTagKey::MusicBrainzArtistId => {}
                StandardTagKey::MusicBrainzDiscId => {}
                StandardTagKey::MusicBrainzGenreId => {}
                StandardTagKey::MusicBrainzLabelId => {}
                StandardTagKey::MusicBrainzOriginalAlbumId => {}
                StandardTagKey::MusicBrainzOriginalArtistId => {}
                StandardTagKey::MusicBrainzRecordingId => {}
                StandardTagKey::MusicBrainzReleaseGroupId => {}
                StandardTagKey::MusicBrainzReleaseStatus => {}
                StandardTagKey::MusicBrainzReleaseTrackId => {}
                StandardTagKey::MusicBrainzReleaseType => {}
                StandardTagKey::MusicBrainzTrackId => {}
                StandardTagKey::MusicBrainzWorkId => {}
                StandardTagKey::Opus => {}
                StandardTagKey::OriginalAlbum => {}
                StandardTagKey::OriginalArtist => {}
                StandardTagKey::OriginalDate => {}
                StandardTagKey::OriginalFile => {}
                StandardTagKey::OriginalWriter => {}
                StandardTagKey::Owner => {}
                StandardTagKey::Part => {}
                StandardTagKey::PartTotal => {}
                StandardTagKey::Performer => {}
                StandardTagKey::Podcast => {}
                StandardTagKey::PodcastCategory => {}
                StandardTagKey::PodcastDescription => {}
                StandardTagKey::PodcastKeywords => {}
                StandardTagKey::Producer => {}
                StandardTagKey::PurchaseDate => {}
                StandardTagKey::Rating => {}
                StandardTagKey::ReleaseCountry => {}
                StandardTagKey::ReleaseDate => {}
                StandardTagKey::Remixer => {}
                StandardTagKey::ReplayGainAlbumGain => {}
                StandardTagKey::ReplayGainAlbumPeak => {}
                StandardTagKey::ReplayGainTrackGain => {}
                StandardTagKey::ReplayGainTrackPeak => {}
                StandardTagKey::Script => {}
                StandardTagKey::SortAlbum => {}
                StandardTagKey::SortAlbumArtist => {}
                StandardTagKey::SortArtist => {}
                StandardTagKey::SortComposer => {}
                StandardTagKey::SortTrackTitle => {}
                StandardTagKey::TaggingDate => {}
                StandardTagKey::TrackNumber => match tag.value {
                    Value::String(val) => {

                        let mut final_val = val;
                        
                        if final_val.contains("/") {
                            final_val = final_val.split("/").next().expect("Number").parse().unwrap();
                        }

                        album_tracks.track_number = final_val.parse::<u64>().expect(format!("Invalid track number: {}", final_val).as_str());


                    }
                    _ => {}
                },
                StandardTagKey::TrackSubtitle => {}
                StandardTagKey::TrackTitle => match tag.value {
                    Value::String(name) => {
                        track.name = Some(name);
                    }
                    _ => {
                        log::error!("Track name is not a string");
                    }
                },
                StandardTagKey::TrackTotal => match tag.value {
                    Value::String(val) => {
                        album.num_of_tracks = val.parse::<u64>().unwrap();
                    }
                    _ => {
                        log::error!("Track number is not a number");
                    }
                },
                StandardTagKey::TvEpisode => {}
                StandardTagKey::TvEpisodeTitle => {}
                StandardTagKey::TvNetwork => {}
                StandardTagKey::TvSeason => {}
                StandardTagKey::TvShowTitle => {}
                StandardTagKey::Url => {}
                StandardTagKey::UrlArtist => {}
                StandardTagKey::UrlCopyright => {}
                StandardTagKey::UrlInternetRadio => {}
                StandardTagKey::UrlLabel => {}
                StandardTagKey::UrlOfficial => {}
                StandardTagKey::UrlPayment => {}
                StandardTagKey::UrlPodcast => {}
                StandardTagKey::UrlPurchase => {}
                StandardTagKey::UrlSource => {}
                StandardTagKey::Version => {}
                StandardTagKey::Writer => {}
            }
        }
    }



    match artist.name {
        Some(name) => {
            match conn.execute(
                "INSERT INTO artists (name) VALUES (?)",
                [&name],
            ) {
                Ok(_) => {
                    log::info!("Added artist {} to artists", name);
                    artist.id = conn.last_insert_rowid() as u64;
                }
                Err(_) => {
                    log::warn!("Artist: {} already created", name);
                    artist.id = conn.query_row(
                        "SELECT id FROM artists WHERE name = ?",
                        &[&name],
                        |row| {
                            row.get::<usize, u32>(0)
                        },
                    ).unwrap() as u64;
                }
            }
        }
        None => {
            log::error!("Artist name is None");
        }
    }
    
    conn.execute(
        "INSERT INTO track (name, path, artist_id) VALUES (?, ?, ?)",
        (&track.name, filepath.to_str().unwrap(), artist.id),
    )
    .unwrap();

    track.id = conn.last_insert_rowid() as u64;


    log::info!("{}", album_tracks.track_number);
    if album_tracks.track_number == 0 {
    } else {
        match conn.execute(
            "INSERT INTO album (name, disc_number, track_number, artist_id) VALUES (?, ?, ?, ?)",
            (&album.name, &album.num_of_discs, &album.num_of_tracks, &album.artist_id),
        ) {
            Ok(_) => {
                log::info!("Added album {} to temp_album", album.name);
                album.id = conn.last_insert_rowid() as u32;
            }
            Err(_) => {
                log::warn!("Album: {} already created", album.name);

                album.id = conn.query_row(
                    "SELECT id FROM album WHERE name = ?",
                    &[&album.name],
                    |row| {
                        row.get::<usize, u32>(0)
                    },
                ).unwrap();
            }
        }



        log::info!(
            "Adding track {} to album {}",
            album_tracks.track_number,
            album.name
        );
        conn.execute(
            "INSERT INTO album_tracks (track_number, disc_number, track_id, album_id) VALUES (?, ?, ?, ?)",
            (&album_tracks.track_number, &album_tracks.disc_number, &track.id, &album.id),
        ).unwrap();
    }
}


