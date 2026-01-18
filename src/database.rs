// SPDX-License-Identifier: GPL-2.0-or-later

use colored::Colorize;
use cosmic::dialog::file_chooser::open::file;
use cosmic::Application;
use regex::{Match, Regex};
use rusqlite::fallible_iterator::FallibleIterator;
use rusqlite::Connection;
use std::fs;
use std::path::PathBuf;
use symphonia::core::meta::{StandardTagKey, Tag, Value};
use symphonia::default::get_probe;

struct Artist {
    id: u32,
    name: Option<String>,
}

struct Album {
    id: u32,
    name: String,
    artist_id: Option<u32>,
    num_of_discs: u32,
    num_of_tracks: u32,
}

struct Track {
    id: u16,
    genres: Option<Vec<String>>,
    name: Option<String>,
}

struct AlbumTracks {
    track_number: u32,
    disc_number: u32,
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
        DROP TABLE IF EXISTS genres;
        DROP TABLE IF EXISTS track_genres;
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
    CREATE TABLE genres (
        id INTEGER PRIMARY KEY,
        name TEXT UNIQUE
    )",
        [],
    )
    .unwrap();

    conn.execute(
        "
    CREATE TABLE track_genres(
        id INTEGER PRIMARY KEY,
        track_id INTEGER,
        genre_id INTEGER,
        FOREIGN KEY(genre_id) REFERENCES genres(id)
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
            name TEXT,
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

    let mut track = Track {
        id: 0,
        genres: None,
        name: None,
    };

    let mut album = Album {
        id: 0,
        name: "".to_string(),
        artist_id: None,
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
                                album.artist_id = Some(conn.last_insert_rowid() as u32);
                            }
                            Err(_) => {
                                // log::warn!("Artist: {} already created", name);
                                album.artist_id = Some(
                                    conn.query_row(
                                        "SELECT id FROM artists WHERE name = ?",
                                        &[&name],
                                        |row| row.get::<usize, u32>(0),
                                    )
                                    .unwrap(),
                                );
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
                            log::info!("final val = {}", final_val);
                            final_val = final_val
                                .split("/")
                                .next()
                                .expect("Number")
                                .parse()
                                .unwrap();
                        }

                        album_tracks.disc_number = final_val
                            .parse::<u32>()
                            .expect(format!("Invalid track number: {}", final_val).as_str());
                    }
                    Value::UnsignedInt(val) => {
                        // log::info!("{}: {}", "DISC NUMBER unsigned int".red(), val);
                        album_tracks.disc_number = val as u32
                    }
                    _ => {
                        // log::error!("DISC NUMBER");
                    }
                },
                StandardTagKey::DiscSubtitle => {}
                StandardTagKey::DiscTotal => match tag.value {
                    Value::String(val) => album.num_of_discs = val.parse::<u32>().unwrap(),
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
                StandardTagKey::Genre => {
                    if !tag.value.to_string().is_empty() {
                        match conn.execute(
                            "insert into genres (name) values (?)",
                            [tag.value.to_string()],
                        ) {
                            Ok(_) => {}
                            Err(err) => {
                                // log::error!("error: {}", err);
                            }
                        }

                        if let Some(genres) = &mut track.genres {
                            genres.push(tag.value.to_string())
                        } else {
                            track.genres = Some(vec![tag.value.to_string()]);
                        }
                    } else {
                    }
                }
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

                        album_tracks.track_number = final_val
                            .parse::<u32>()
                            .expect(format!("Invalid track number: {}", final_val).as_str());
                    }
                    Value::UnsignedInt(val) => album_tracks.track_number = val as u32,

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
                        album.num_of_tracks = val.parse::<u32>().unwrap();
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

    log::info!(
        " {} BY {} IN {}",
        &track.name.as_ref().unwrap().on_blue(),
        &artist.name.as_ref().unwrap().on_bright_yellow().black(),
        &album.name.on_bright_blue()
    );

    match artist.name {
        Some(name) => match conn.execute("INSERT INTO artists (name) VALUES (?)", [&name]) {
            Ok(_) => {
                // log::info!("Added artist {} to artists", name);
                artist.id = conn.last_insert_rowid() as u32;
            }
            Err(_) => {
                // log::warn!("Artist: {} already created", name);
                artist.id = conn
                    .query_row("SELECT id FROM artists WHERE name = ?", &[&name], |row| {
                        row.get::<usize, u32>(0)
                    })
                    .unwrap();
            }
        },
        None => {
            // log::error!("No artist was found");
        }
    }

    conn.execute(
        "INSERT INTO track (name, path, artist_id) VALUES (?, ?, ?)",
        (&track.name, filepath.to_str().unwrap(), artist.id),
    )
    .unwrap();

    track.id = conn.last_insert_rowid() as u16;

    if let Some(genres) = track.genres {
        for genre in genres {
            match conn.query_row(
                "SELECT id FROM genres WHERE name = ?",
                &[&genre.trim().to_string()],
                |row| {
                    let row_id = row.get::<usize, u32>(0).unwrap();

                    log::info!(
                        "Genre: {} to be inserted with track id: {}",
                        genre,
                        track.id
                    );
                    match conn.execute(
                        "insert into track_genres (track_id, genre_id) values (?, ?)",
                        [track.id, (row_id as u16).into()],
                    ) {
                        Ok(v) => {
                            log::info!("TRACKID: {}", track.id);
                            Ok(v)
                        }
                        Err(err) => {
                            log::error!("error while inserting into genre_tracks: {}", err);
                            Ok(3)
                        }
                    }
                },
            ) {
                Ok(_) => {}
                Err(err) => {
                    log::error!("error while finding id from genre_name{}", err)
                }
            }
        }
    }

    if album.name.is_empty() {
        log::warn!("{}", "No album title in file mdat".red())
    } else {
        // Check how many albums exist with this name
        let a_similar: Vec<(u32, Option<u32>)>;

        if album.artist_id.is_some() {
            let mut similar_albums = match conn.prepare(
                "
                select main.album.id as id, art.id as aid from main.album
                    join main.artists art on main.album.artist_id = art.id
                where main.album.name = ?
            ",
            ) {
                Ok(val) => val,
                Err(err) => {
                    log::error!("{}", err);
                    panic!("Faulty sql request")
                }
            };

            let a_iter = similar_albums
                .query_map(&[&album.name], |row| {
                    Ok((
                        row.get::<&str, u32>("id").unwrap(),
                        Some(row.get::<&str, u32>("aid").unwrap()),
                    ))
                })
                .unwrap();

            a_similar = a_iter
                .into_iter()
                .filter_map(|a| a.ok())
                .collect::<Vec<(u32, Option<u32>)>>();
        } else {
            let mut similar_albums = match conn.prepare(
                "
                select main.album.id as id from main.album
                where main.album.name = ?
            ",
            ) {
                Ok(val) => val,
                Err(err) => {
                    log::error!("{}", err);
                    panic!("Faulty sql request")
                }
            };

            let a_iter = similar_albums
                .query_map(&[&album.name], |row| {
                    Ok((row.get::<&str, u32>("id").unwrap(), None))
                })
                .unwrap();

            a_similar = a_iter
                .into_iter()
                .filter_map(|a| a.ok())
                .collect::<Vec<(u32, Option<u32>)>>();
        }

        log::info!("Similarities: {}", a_similar.len());

        if a_similar.len() > 0 {
            // if at least one album already exists with the same name, compare artists to verify if it is actually the same album
            let mut insertion = false;
            for artist_id in a_similar {
                if artist_id.1.is_none() {
                    // if there is no artist value associated, but there is an album with the same name, insert anyway it's probably correct
                    album.id = artist_id.0;
                    insertion = true;
                } else {
                    if artist_id.1.unwrap() == album.artist_id.unwrap() as u32 {
                        // if album name is the same and artist_id this is probably the correct album
                        album.id = artist_id.0;
                        insertion = true;
                    }
                }
            }

            if insertion == false {
                // if artist id does not match with any previous album entries, it probably is a different album
                let image_dat = find_visual(filepath);
                insert_track_to_grouping(&album, track.id as u32, image_dat, &conn);
                album.id = conn.last_insert_rowid() as u32
            }
        } else {
            // if there are no matching albums create a new one, or if there is only one track associated, assume it is a single
            let image_dat = find_visual(filepath);
            insert_track_to_grouping(&album, track.id as u32, image_dat, &conn);
            album.id = conn.last_insert_rowid() as u32
        }

        if album.num_of_tracks != 1 {
            match conn.execute(
                "INSERT INTO album_tracks (album_id, track_id, track_number, disc_number) VALUES (?, ?, ?, ?)",
                (&album.id, &track.id, &album_tracks.track_number, &album_tracks.disc_number),
            ) {
                Ok(_) => {

                }
                Err(err) => {
                    log::error!("album_track insertion went wrong \n ------ \n  {}", err.to_string());
                }
            }
        }
    }
}

fn insert_track_to_grouping(
    album: &Album,
    track_id: u32,
    image_dat: Option<Box<[u8]>>,
    conn: &Connection,
) {
    if album.num_of_tracks != 1 {
        //Album
        match conn.execute(
            "INSERT INTO album (name, disc_number, track_number, artist_id, album_cover) VALUES (?, ?, ?, ?, ?)",
            (&album.name, &album.num_of_discs, &album.num_of_tracks, &album.artist_id, image_dat),
        ) {
            Ok(_) => {
                log::info!("{}", "Successfully added ALBUM!".purple());
            }
            Err(err) => {
                log::error!("{} \n ERROR: {}", "Failed to insert ALBUM".red(), err);

            }
        }
    } else {
        //Single
        match conn.execute(
            "INSERT INTO single (track_id, cover) VALUES (?, ?)",
            (&track_id, image_dat),
        ) {
            Ok(_) => {
                log::info!("{}", "Successfully added SINGLE!".green());
            }
            Err(err) => {
                log::error!("{} \n ERROR: {}", "Failed to insert SINGLE".red(), err);
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
