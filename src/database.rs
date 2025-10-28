// SPDX-License-Identifier: GPL-2.0-or-later

use colored::Colorize;
use cosmic::Application;
use regex::{Match, Regex};
use std::fs;
use std::path::PathBuf;
use symphonia::core::meta::{StandardTagKey, Tag, Value};
use symphonia::default::get_probe;

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
}

struct Track {
    id: u64,
    name: Option<String>,
}

struct AlbumTracks {
    track_number: u64,
    disc_number: u64,
}

pub fn create_database() {
    let conn = rusqlite::Connection::open(
        dirs::data_local_dir()
            .unwrap()
            .join(crate::app::AppModel::APP_ID)
            .join("nova_music.db"),
    )
    .unwrap();

    conn.execute_batch(
        "
        DROP TABLE IF EXISTS temp_album;
        DROP TABLE IF EXISTS album;
        DROP TABLE IF EXISTS album_tracks;
        DROP TABLE IF EXISTS track;
        DROP TABLE IF EXISTS single
    ",
    )
    .unwrap();

    conn.execute(
        "
    CREATE TABLE if not exists artists (
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE,
        artistpfp BLOB
    )
    ",
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
        FOREIGN KEY(album_id) REFERENCES album(id),
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
            album_cover BLOB,
            FOREIGN KEY(artist_id) REFERENCES artist(id)
        )",
        [],
    )
    .unwrap();

    conn.execute(
        "
        CREATE TABLE single (
            id INTEGER PRIMARY KEY,
            track_id INTEGER,
            cover BLOB,
            FOREIGN KEY(track_id) REFERENCES tracks(id)
        )",
        [],
    )
    .unwrap();
}
//todo: Theres probably a better way to do this.
pub async fn create_database_entry(metadata_tags: Vec<Tag>, filepath: &PathBuf) {
    let conn = rusqlite::Connection::open(
        dirs::data_local_dir()
            .unwrap()
            .join(crate::app::AppModel::APP_ID)
            .join("nova_music.db"),
    )
    .unwrap();

    let mut track = Track { id: 0, name: None };

    let mut album = Album {
        id: 0,
        name: "".to_string(),
        artist_id: 0,
        num_of_discs: 1,
        num_of_tracks: 0,
    };

    let mut album_tracks = AlbumTracks {
        disc_number: 0,
        track_number: 0,
    };

    let mut artist = Artist { id: 0, name: None };

    for tag in metadata_tags {
        if let Some(key) = tag.std_key {
            match key {
                //todo: maybe one day account for most of these tags somewhere
                StandardTagKey::AcoustidFingerprint => {}
                StandardTagKey::AcoustidId => {}
                StandardTagKey::Album => match tag.value {
                    Value::String(name) => {
                        // log::info!("This file maay be a part of an album!");
                        album.name = name;
                    }
                    _ => {
                        // log::error!("Album name is not a string");
                    }
                },
                StandardTagKey::AlbumArtist => match tag.value {
                    Value::String(mut name) => {
                        // let regex = Regex::new("/Feat.|ft.|&/i").unwrap();
                        //
                        // match regex.find(&name) {
                        //     None => {}
                        //     Some(val) => {
                        //
                        //         name.truncate(val.start());
                        //     }
                        // };
                        //

                        match conn.execute("INSERT INTO artists (name) VALUES (?)", [name.trim()]) {
                            Ok(_) => {
                                // log::info!("Added artist {} to artists", name);
                                album.artist_id = conn.last_insert_rowid() as u64;
                            }
                            Err(_) => {
                                // log::warn!("Artist: {} already created", name);
                                album.artist_id =
                                    conn.query_row(
                                        "SELECT id FROM artists WHERE name = ?",
                                        &[&name],
                                        |row| row.get::<usize, u32>(0),
                                    )
                                    .unwrap() as u64;
                                // log::info!("ARTIST ID:  {}", album.artist_id);
                            }
                        }
                    }
                    _ => {}
                },
                StandardTagKey::Arranger => {}
                StandardTagKey::Artist => {
                    if let Value::String(val) = tag.value {
                        artist.name = Some(val)
                    }
                }
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
                        // log::info!("DISC NUMBER");
                        let mut final_val = val;

                        if final_val.contains("/") {
                            final_val = final_val
                                .split("/")
                                .next()
                                .expect("Number")
                                .parse()
                                .unwrap();
                        }

                        album_tracks.disc_number = final_val
                            .parse::<u64>()
                            .expect(format!("Invalid track number: {}", final_val).as_str());
                    }
                    Value::UnsignedInt(val) => {
                        // log::info!("{}: {}", "DISC NUMBER unsigned int".red(), val);
                        album_tracks.disc_number = val
                    }
                    _ => {
                        // log::error!("DISC NUMBER");
                    }
                },
                StandardTagKey::DiscSubtitle => {}
                StandardTagKey::DiscTotal => match tag.value {
                    Value::String(val) => album.num_of_discs = val.parse::<u64>().unwrap(),
                    _ => {
                        // log::error!("Disc number is not a number");
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
                            final_val = final_val
                                .split("/")
                                .next()
                                .expect("Number")
                                .parse()
                                .unwrap();
                        }
                        log::info!("FINAL VAL: {}", final_val.on_red());

                        album_tracks.track_number = final_val
                            .parse::<u64>()
                            .expect(format!("Invalid track number: {}", final_val).as_str());
                    }
                    Value::UnsignedInt(val) => album_tracks.track_number = val,

                    Value::Binary(_) => {
                        // log::info!("{}", "TRACK NUMBER binary".red());
                    }
                    Value::Boolean(_) => {
                        // log::info!("{}", "TRACK NUMBER  boolean".red());
                    }
                    Value::Flag => {
                        // log::info!("{}", "TRACK NUMBER  flag".red());
                    }
                    Value::Float(_) => {
                        // log::info!("{}", "TRACK NUMBER  float".red());
                    }
                    Value::SignedInt(_) => {
                        // log::info!("{}", "TRACK NUMBER  signed int".red());
                    }
                },
                StandardTagKey::TrackSubtitle => {}
                StandardTagKey::TrackTitle => match tag.value {
                    Value::String(name) => {
                        track.name = Some(name);
                    }
                    _ => {
                        // log::error!("Track name is not a string");
                    }
                },
                StandardTagKey::TrackTotal => match tag.value {
                    Value::String(val) => {
                        album.num_of_tracks = val.parse::<u64>().unwrap();
                    }
                    _ => {
                        // log::error!("Track number is not a number");
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
        Some(name) => match conn.execute("INSERT INTO artists (name) VALUES (?)", [&name]) {
            Ok(_) => {
                // log::info!("Added artist {} to artists", name);
                artist.id = conn.last_insert_rowid() as u64;
            }
            Err(_) => {
                // log::warn!("Artist: {} already created", name);
                artist.id = conn
                    .query_row("SELECT id FROM artists WHERE name = ?", &[&name], |row| {
                        row.get::<usize, u32>(0)
                    })
                    .unwrap() as u64;
            }
        },
        None => {
            // log::error!("Artist name is None");
        }
    }

    conn.execute(
        "INSERT INTO track (name, path, artist_id) VALUES (?, ?, ?)",
        (&track.name, filepath.to_str().unwrap(), artist.id),
    )
    .unwrap();

    track.id = conn.last_insert_rowid() as u64;

    // log::info!("{}", album.name.on_red());
    if album.name.is_empty() {
    } else {
        // If album already exists, no need to add extra info
        // log::info!("Looking to insert {}", album.name);
        match conn.query_row(
            "select id from album where name = ?",
            &[&album.name],
            |row| row.get::<usize, u32>(0),
        ) {
            Ok(val) => {
                // log::info!(
                //     "Album with title, {}, found \n {}",
                //     album.name.white().on_blue().bold(),
                //     val
                // );
                // Album already exists
                album.id = val
            }
            Err(_err) => {
                // log::info!(
                //     "No album with title, {}, found; Creating a new one \n ------ \n {}",
                //     album.name.white().on_blue().bold(),
                //     err.to_string()
                // );
                // Album does not exist yet

                if let Some(visual) = find_visual(filepath) {
                    //If visual data exists

                    if album.num_of_tracks == 1 {
                        match conn.execute(
                            "INSERT INTO single (track_id, cover) VALUES (?, ?)",
                            (&track.id, &visual),
                        ) {
                            Ok(_) => {
                                log::info!("{}", "Added SINGLE with some visual!".green());
                            }
                            Err(err) => {
                                log::error!("{}", "UNABLE TO INSERT *SINGLE* DATA W/ VISUAL".red());
                                panic!("{}", err)
                            }
                        }
                    } else {
                        match conn.execute(
                        "INSERT INTO album (name, disc_number, track_number, artist_id, album_cover) VALUES (?, ?, ?, ?, ?)",
                        (&album.name, &album.num_of_discs, &album.num_of_tracks, &album.artist_id, &visual),
                    ) {
                        Ok(_) => {
                            log::info!("{}", "Added ALBUM with some visual!".green());
                        }
                        Err(_) => {
                            log::error!("{}", "UNABLE TO INSERT ALBUM DATA W/ VISUAL".red());
                        }
                    }
                    }
                } else {
                    //If visual data does not exist
                    if album.num_of_tracks == 1 {
                        match conn.execute(
                            "INSERT INTO single (track.id, cover) VALUES (?, ?)",
                            (&track.id, None::<Box<[u8]>>),
                        ) {
                            Ok(_) => {
                                log::info!("{}", "Added SINGLE with some visual!".green());
                            }
                            Err(_) => {
                                log::error!("{}", "UNABLE TO INSERT *SINGLE* DATA W/ VISUAL".red());
                            }
                        }
                    } else {
                        match conn.execute(
                        "INSERT INTO album (name, disc_number, track_number, artist_id, album_cover) VALUES (?, ?, ?, ?, ?)",
                        (&album.name, &album.num_of_discs, &album.num_of_tracks, &album.artist_id, None::<Box<[u8]>>),
                    ) {
                        Ok(_) => {
                            // log::info!("{}", "Added album without visual!".purple());
                        }
                        Err(_err) => {
                            // log::error!("{} \n {}", "UNABLE TO INSERT ALBUM DATA W/O VISUAL".red(), err.to_string());

                        }
                    }
                    }
                }
                album.id = conn.last_insert_rowid() as u32
            }
        }

        if album.num_of_tracks != 1 {
            match conn.execute(
                "INSERT INTO album_tracks (album_id, track_id, track_number, disc_number) VALUES (?, ?, ?, ?)",
                (&album.id, &track.id, &album_tracks.track_number, &album_tracks.disc_number),
            ) {
                Ok(_) => {

                }
                Err(_err) => {
                    // log::error!("album_track insertion went wrong \n ------ \n  {}", err.to_string());
                }
            }
        }
    }
}

pub fn find_visual(filepath: &PathBuf) -> Option<Box<[u8]>> {
    let file = fs::File::open(filepath).unwrap();

    let probe = get_probe();
    let mss = symphonia::core::io::MediaSourceStream::new(Box::new(file), Default::default());

    let mut reader = match probe.format(
        &Default::default(),
        mss,
        &Default::default(),
        &Default::default(),
    ) {
        Ok(read) => read,
        Err(err) => {
            panic!("{}", err.to_string());
        }
    };

    if let Some(mdat_rev) = reader.metadata.get() {
        if let Some(mdat_rev) = mdat_rev.current() {
            match mdat_rev.visuals().get(0) {
                Some(visual) => {
                    // log::info!("This album contains visual data!");
                    Some(visual.data.clone())
                }
                None => {
                    // log::info!("This album contains no visual data!");
                    None
                }
            }
        } else {
            None
        }
    } else {
        if let Some(mdat_rev) = reader.format.metadata().current() {
            Some(mdat_rev.visuals().get(0)?.data.clone())
        } else {
            None
        }
    }
}
