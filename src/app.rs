// SPDX-License-Identifier: GPL-2.0-or-later

mod albums;
mod artists;
pub(crate) mod home;
mod playlists;
mod scan;
mod settings;
mod tracks;

use std::any::TypeId;
use tokio::task::{spawn_blocking, JoinHandle};

use crate::app::albums::{
    get_album_info, get_top_album_info, Album, AlbumPage, AlbumPageState, FullAlbum,
};


use crate::app::home::HomePage;
use crate::app::tracks::TrackPage;
use crate::app::scan::{scan_directory, MediaFileTypes};

use crate::app::Message::{
    AddTrackToSink, AlbumPageStateAlbum, AlbumProcessed, AlbumRequested, SongFinished,
    StreamPaused, UpdateScanProgress,
};
use crate::config::Config;
use crate::database::{create_database, create_database_entry};
use crate::{app, config, fl, StandardTagKeyExt};
use colored::Colorize;
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, ConfigSet, CosmicConfigEntry};
use cosmic::cosmic_theme::palette::cast::IntoComponents;
use cosmic::cosmic_theme::palette::chromatic_adaptation::AdaptInto;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::event::Status;
use cosmic::iced::keyboard::key::Code::Home;
use cosmic::iced::keyboard::key::Physical::Code;
use cosmic::iced::wgpu::naga::back::spv::Capability::MeshShadingEXT;
use cosmic::iced::wgpu::naga::FastHashMap;
use cosmic::iced::wgpu::Queue;
use cosmic::iced::window::Id;
use cosmic::iced::{
    alignment, event, stream, Alignment, ContentFit, Event, Fill, Length, Pixels, Subscription,
};
use cosmic::iced_core::text::Wrapping;
use cosmic::iced_core::widget::operation::map;
use cosmic::iced_wgpu::window::compositor::new;
use cosmic::prelude::*;
use cosmic::widget::segmented_button::Entity;
use cosmic::widget::{self, container, icon, menu, nav_bar, progress_bar, toaster, JustifyContent};
use cosmic::{cosmic_theme, iced, iced_futures, theme};
use futures::channel::mpsc;
use futures::channel::mpsc::{Receiver, Sender, TrySendError};
use futures_util::stream::{Next, SelectNextSome};
use futures_util::{SinkExt, StreamExt};
use log::info;
use rodio::source::SeekError::SymphoniaDecoder;

use rodio::{OutputStream, Sink, Source};
use rusqlite::fallible_iterator::FallibleIterator;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::thread::sleep;
use std::time::Duration;
use cosmic::iced::task::Handle;
use rodio::source::SeekError;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_AAC, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, StandardTagKey, Tag, Value};
use symphonia::core::probe::Hint;
use symphonia::core::units::Time;
use symphonia::default::get_probe;

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] = include_bytes!("../resources/icons/hicolor/scalable/apps/icon.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,
    /// Display a context drawer with the designated page if defined.
    context_page: ContextPage,
    /// Contains items assigned to the nav bar panel.
    nav: nav_bar::Model,
    /// Key bindings for the application's menu bar.
    key_binds: HashMap<menu::KeyBind, Action>,
    // Configuration data that persists between application runs.
    config: Config,
    config_handler: cosmic_config::Config,

    //Settings Page
    pub change_dir_filed: String,
    pub rescan_available: bool,

    //Audio
    pub stream: OutputStream,
    pub sink: Arc<Sink>,
    pub loop_state: LoopState,
    pub song_progress: f64,
    pub song_duration: Option<f64>,
    pub queue: Vec<AppTrack>,
    pub queue_pos: usize,
    pub clear: bool,
    pub task_handle: Option<Vec<Handle>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppTrack {
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub path_buf: PathBuf,
    pub cover_art: Option<cosmic::widget::image::Handle>,
}
/// Messages emitted by the application and its widgets.

pub enum LoopState {
    LoopingTrack,
    LoopingQueue,
    NotLooping,
}
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    StreamPaused,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    EditInput(String),

    // Config change related
    RescanDir,

    // Filesystem scan related
    ChangeScanDir(String),
    UpdateScanProgress(u32),
    UpdateScanDirSize(u32),

    // Page Rendering
    OnNavEnter,

    // Album Page
    AlbumRequested((String, String)), // when an album icon is clicked [gets title & artist of album]
    AlbumInfoRetrieved(FullAlbum), // when task assigned to retrieving requested albums info is completed [gets full track list of album]
    AlbumProcessed(Album), // when an album retrieved from db's data is organized and ready [Supplies AlbumPage with the new Album]
    AlbumsLoaded, // when albums table retrieved from db is exhausted after OnNavEnter in Album Page [Sets page state to loaded]
    AlbumPageStateAlbum(AlbumPage), // when album info is retrieved [Replaces AlbumPage with AlbumPage with new info] todo: Might be able to use this weird implementation to cache one album visit
    AlbumPageReturn,

    // Home Page (Or Now Playing Page idk tbh)
    //todo Change to pathbufs for safety?
    AddTrackToQueue(String),
    //todo Make albums in queue fancier kinda like Elisa does it
    AddAlbumToQueue(Vec<String>),

    // Audio Messages
    PlayPause,
    SongFinished(QueueUpdateReason),
    AddTrackToSink(String),
    ChangeLoopState,
    PreviousTrack,
    SeekFinished,
    ClearQueue,
    SinkProgress(f64),
    SeekTrack(f64),

    // Media Controls
    SkipTrack,

    // Settings
    GridSliderChange(u32),

    //experimenting
    CreatePlaylist,
    ChangeActiveInQueue(usize),
    RemoveSongInQueue(usize)
}

#[derive(Clone, Debug)]
enum QueueUpdateReason {
    Skipped,
    Previous,
    Removed(usize),
    None,
    ThreadKilled,
}

/// Create a COSMIC application from the app model
impl cosmic::Application for AppModel {
    /// The async executor that will be used to run your application's commands.
    type Executor = cosmic::executor::Default;

    /// Data that your application receives to its init method.
    type Flags = ();

    /// Messages which the application and its widgets will emit.
    type Message = Message;

    /// Unique identifier in RDNN (reverse domain name notation) format.
    const APP_ID: &'static str = "dev.riveroluna.cosmicmusic";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

    // fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
    //
    // }

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();
        let stream =
            rodio::OutputStreamBuilder::open_default_stream().expect("Failed to open stream");
        let sink = rodio::Sink::connect_new(stream.mixer());

        let sink = Arc::new(sink);
        nav.insert()
            .text(fl!("home"))
            .data::<Page>(Page::NowPlaying(HomePage))
            .icon(icon::from_name("applications-audio-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("tracks"))
            .data::<Page>(Page::Tracks(TrackPage))
            .icon(icon::from_name("media-tape-symbolic"));

        nav.insert()
            .text(fl!("albums"))
            .data::<Page>(Page::Albums(AlbumPage::new(None)))
            .icon(icon::from_name("media-optical-symbolic"));

        nav.insert()
            .text(fl!("artists"))
            .data::<Page>(Page::Artists)
            .icon(icon::from_name("avatar-default-symbolic"));



        // todo Add playlist support
        nav.insert()
            .text(fl!("playlists"))
            .data::<Page>(Page::Playlists)
            .icon(icon::from_name("playlist-symbolic"));

        // INIT CONFIG
        let config = config::Config::load();
        let config_handler = match config.0 {
            None => {
                panic!("NO CONFIG");
            }
            Some(som) => som,
        };
        let config = config.1;

        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),
            // Optional configuration file for an application.
            config,
            config_handler,
            change_dir_filed: "".to_string(),
            rescan_available: true,
            // Audio
            stream,
            sink,
            loop_state: LoopState::NotLooping,
            song_progress: 0.0,
            song_duration: None,
            queue: vec![],
            queue_pos: 0,
            clear: false,
            task_handle: None,
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")).apply(Element::from),
            menu::items(
                &self.key_binds,
                vec![
                    menu::Item::Button(fl!("settings"), None, Action::Settings),
                    menu::Item::Button(fl!("about"), None, Action::About),
                ],
            ),
        )]);

        vec![menu_bar.into()]
    }

    fn on_close_requested(&self, id: Id) -> Option<Self::Message> {
        self.sink.stop();
        match &self.task_handle {
            None => {}
            Some(handles) => {
                for handle in handles {
                    handle.abort()
                }
            }
        }
        None
    }

    /// Enables the COSMIC application to create a nav bar with this model.
        fn nav_model(&self) -> Option<&nav_bar::Model> {
            Some(&self.nav)
        }

        /// Display a context drawer if the context page is requested.
        fn context_drawer(&self) -> Option<context_drawer::ContextDrawer<Self::Message>> {
            if !self.core.window.show_context {
                return None;
            }

            Some(match self.context_page {
                ContextPage::About => context_drawer::context_drawer(
                    self.about(),
                    Message::ToggleContextPage(ContextPage::About),
                )
                    .title(fl!("about")),
                ContextPage::Settings => context_drawer::context_drawer(
                    self.settings(),
                    Message::ToggleContextPage(ContextPage::Settings),
                )
                    .title(fl!("settings")),
            })
        }

        /// Describes the interface based on the current state of the application model.
        ///
        /// Application events will be processed through the view. Any messages emitted by
        /// events received by widgets will be passed to the update method.
        fn view(&self) -> Element<Self::Message> {
            match self.nav.active_data::<Page>().unwrap() {
                Page::NowPlaying(home_page) => home_page.load(&self),
                Page::Tracks(track_page) => {
                    track_page.load()
                }
                Page::Artists => {
                    cosmic::widget::container::Container::new(
                        cosmic::widget::text::title1(" Artists ")
                    )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
                Page::Albums(album_page) => {
                    album_page.load_page(&self.config.grid_item_size, &self.config.grid_item_spacing)
                }
                Page::Playlists => {
                    cosmic::widget::container::Container::new(
                        cosmic::widget::text::title1(" Playlists ")
                    )
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .into()
                }
            }
        }

        /// Handles messages emitted by the application and its widgets.
        ///
        /// Tasks may be returned for asynchronous execution of code in the background
        /// on the application's async runtime.
        fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
            match message {
                Message::OpenRepositoryUrl => {
                    _ = open::that_detached(REPOSITORY);
                }
                Message::ChangeActiveInQueue(index) => {
                    self.clear = true;
                    self.sink.clear();
                    self.sink.play();
                    self.queue_pos = index;
                }
                Message::RemoveSongInQueue(index) => {
                    return cosmic::task::future(async move {
                        Message::SongFinished(QueueUpdateReason::Removed(index))
                    });
                }
                Message::ChangeLoopState => match self.loop_state {
                    LoopState::LoopingTrack => {
                        self.loop_state = LoopState::NotLooping;
                    }
                    LoopState::LoopingQueue => {
                        self.loop_state = LoopState::LoopingTrack;
                    }
                    LoopState::NotLooping => {
                        self.loop_state = LoopState::LoopingQueue;
                    }
                },
                Message::StreamPaused => {
                    log::info!("{}", "Stream Paused At".red());
                }

                Message::ToggleContextPage(context_page) => {
                    if self.context_page == context_page {
                        // Close the context drawer if the toggled context page is the same.
                        self.core.window.show_context = !self.core.window.show_context;
                    } else {
                        // Open the context drawer to display the requested context page.
                        self.context_page = context_page;
                        self.core.window.show_context = true;
                    }
                }

                Message::EditInput(val) => {
                    self.change_dir_filed = val.to_string();
                }
                Message::ChangeScanDir(val) => match fs::read_dir(&val) {
                    Ok(dir) => match self.config.set_scan_dir(&self.config_handler, val) {
                        Ok(val) => {}
                        Err(err) => {
                            log::error!("dir: {:?}", err);
                        }
                    },
                    Err(error) => {}
                },

                Message::UpdateConfig(config) => {
                    self.config = config;
                }

                Message::LaunchUrl(url) => match open::that_detached(&url) {
                    Ok(()) => {}
                    Err(err) => {
                        eprintln!("failed to open {url:?}: {err}");
                    }
                },
                Message::RescanDir => {

                    self.clear = true;
                    self.sink.stop();
                    match &self.task_handle {
                        None => {}
                        Some(handles) => {
                            for handle in handles {
                                handle.abort()
                            }
                        }
                    }

                    self.queue_pos = 0;
                    self.song_progress = 0.0;
                    self.song_duration = None;

                    self.queue.clear();

                    // Settings: No rescan until current rescan finishes
                    self.rescan_available = false;
                    self.config.set_num_files_found(&self.config_handler, 0);
                    self.config.set_files_scanned(&self.config_handler, 0);

                    // Albums: Full reset
                    let album_pos = self.nav.entity_at(2).unwrap();
                    let album_dat = self.nav.data_mut::<Page>(album_pos).unwrap();
                    if let Page::Albums(page) = album_dat {
                        page.albums = None;
                        page.page_state = AlbumPageState::Loading
                    }

                    create_database();

                    let path = self.config.scan_dir.clone().parse().unwrap();
                    return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                        0,
                        |mut tx| async move {
                            let files = scan_directory(path, &mut tx).await;
                            let mut files_scanned = 0;

                            for file in files {
                                files_scanned += 1;
                                match file {
                                    MediaFileTypes::FLAC(path) => {
                                        log::info!("File: {:?}", path);
                                        let file = fs::File::open(&path).unwrap();

                                        let probe = get_probe();
                                        let mss = symphonia::core::io::MediaSourceStream::new(
                                            Box::new(file),
                                            Default::default(),
                                        );

                                        let mut hint = Hint::new();
                                        hint.with_extension("flac");

                                        let mut reader = match probe.format(
                                            &hint,
                                            mss,
                                            &Default::default(),
                                            &Default::default(),
                                        ) {
                                            Ok(read) => read,
                                            Err(err) => {
                                                eprintln!("{}", err);
                                                continue;
                                            }
                                        }
                                            .format;

                                        if let Some(metadata_rev) = reader.metadata().current() {
                                            let metadata_tags = metadata_rev
                                                .tags()
                                                .into_iter()
                                                .filter(|val| val.is_known())
                                                .map(|val| val.clone())
                                                .collect::<Vec<Tag>>();

                                            create_database_entry(metadata_tags, &path)
                                        } else {
                                            log::info!("no metadata found")
                                        }
                                    }
                                    MediaFileTypes::MP4(path) => {}
                                    MediaFileTypes::MP3(path) => {
                                        log::info!("File: {:?}", path);
                                        let file = fs::File::open(&path).unwrap();

                                        let probe = get_probe();
                                        let mss = symphonia::core::io::MediaSourceStream::new(
                                            Box::new(file),
                                            Default::default(),
                                        );

                                        let mut hint = Hint::new();
                                        hint.with_extension("mp3");

                                        let mut reader = match probe.format(
                                            &hint,
                                            mss,
                                            &Default::default(),
                                            &Default::default(),
                                        ) {
                                            Ok(read) => read,
                                            Err(err) => {
                                                eprintln!("{}", err);
                                                continue;
                                            }
                                        };

                                        if let Some(mdat_revision) = reader.metadata.get() {
                                            if let Some(mdat_revision) = mdat_revision.current() {
                                                let metadata_tags = mdat_revision
                                                    .tags()
                                                    .iter()
                                                    .filter(|a| a.is_known())
                                                    .map(|a| a.clone())
                                                    .collect::<Vec<Tag>>();
                                                create_database_entry(metadata_tags, &path)
                                            }
                                        }
                                    }
                                }
                            }

                            tx.send(Message::UpdateScanProgress(files_scanned))
                                .await
                                .expect("TODO: panic message");
                            tx.send(Message::OnNavEnter).await.expect("de")
                        },
                    ))
                        // Must wrap our app type in `cosmic::Action`.
                        .map(cosmic::Action::App);
                }
                Message::UpdateScanProgress(num) => {
                    self.config.set_files_scanned(&self.config_handler, num);
                    if self.config.files_scanned == self.config.num_files_found {
                        self.rescan_available = true;
                    }
                }

                Message::UpdateScanDirSize(num) => {
                    self.config
                        .set_num_files_found(&self.config_handler, num)
                        .expect("Config Save Failed");
                }

                // PAGE TASK RESPONSES
                Message::AlbumProcessed(new_album) => {
                    let dat_pos = self.nav.entity_at(2).expect("REASON");
                    let data = self.nav.data_mut::<Page>(dat_pos).unwrap();
                    if let Page::Albums(dat) = data {
                        match &mut dat.albums {
                            None => {
                                dat.albums = Some(vec![new_album]);
                            }
                            Some(ref mut val) => {
                                val.push(new_album);
                            }
                        }
                    }
                }
                Message::OnNavEnter => match self.nav.active_data_mut().unwrap() {
                    Page::NowPlaying(HomePage) => {}
                    Page::Artists => {}
                    Page::Albums(val) => {
                        match &mut val.page_state {
                            AlbumPageState::Loading => {}
                            AlbumPageState::Album(page) => {
                                return cosmic::Task::done(Message::AlbumPageReturn)
                                    .map(cosmic::Action::from)
                            }
                            AlbumPageState::Loaded => return Task::none(),
                        }

                        let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();

                        let mut stmt = conn
                            .prepare(
                                "SELECT a.id, a.name,
                        a.disc_number, a.track_number, a.album_cover, art.name as artist_name
                     FROM album a
                     JOIN artists art ON a.artist_id = art.id",
                            )
                            .expect("error preparing sql");

                        let album_iter = stmt
                            .query_map([], |row| {
                                Ok((
                                    row.get::<_, String>("name").unwrap_or_default(),
                                    row.get::<_, String>("artist_name").unwrap_or_default(),
                                    row.get::<_, u32>("disc_number").unwrap_or(0),
                                    row.get::<_, u32>("track_number").unwrap_or(0),
                                    match row.get::<_, Vec<u8>>("album_cover") {
                                        Ok(val) => Some(val),
                                        Err(e) => {
                                            log::info!("{}", e);
                                            None
                                        }
                                    },
                                ))
                            })
                            .expect("error executing query");

                        let albums: Vec<(String, String, u32, u32, Option<Vec<u8>>)> =
                            album_iter.filter_map(|a| a.ok()).collect();

                        return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                            0,
                            |mut tx| async move {
                                get_top_album_info(&mut tx, albums).await;
                                tx.send(Message::AlbumsLoaded).await.expect("de")
                            },
                        ))
                            .map(cosmic::Action::App);

                        // return cosmic::task::future(async move {
                        //
                        // });
                    }
                    Page::Playlists => {},
                    Page::Tracks(_) => {}
                },
                Message::AlbumsLoaded => {
                    let dat_pos = self.nav.entity_at(2).expect("REASON");
                    let data = self.nav.data_mut::<Page>(dat_pos).unwrap();

                    if let Page::Albums(dat) = data {
                        dat.page_state = AlbumPageState::Loaded;
                        dat.has_fully_loaded = true;
                    }
                }
                Message::AlbumPageReturn => {
                    let dat_pos = self.nav.entity_at(2).expect("REASON");
                    let data = self.nav.data_mut::<Page>(dat_pos).unwrap();

                    if let Page::Albums(dat) = data {
                        match dat.has_fully_loaded {
                            true => {
                                dat.page_state = AlbumPageState::Loaded;
                            }
                            false => {
                                dat.page_state = AlbumPageState::Loading;
                            }
                        }
                    }
                }
                Message::AlbumPageStateAlbum(new_page) => {
                    match self.nav.active_data_mut::<Page>().unwrap() {
                        Page::NowPlaying(home_page) => {}
                        Page::Artists => {}
                        Page::Albums(old_page) => {
                            *old_page = new_page;
                        }
                        Page::Playlists => {},
                        Page::Tracks(page) => {},
                    }
                }
                Message::AlbumInfoRetrieved(albuminfopage) => {
                    let pos = self.nav.entity_at(2).expect("REASON");
                    let album_page = self.nav.data_mut::<Page>(pos).unwrap();
                    if let Page::Albums(page) = album_page {
                        page.page_state = AlbumPageState::Album(albuminfopage);
                    }
                }
                Message::AlbumRequested(dat) => {
                    match self.nav.active_data_mut::<Page>().unwrap() {
                        Page::Albums(page_dat) => {
                            return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                                0,
                                |mut tx| async move {
                                    let album = get_album_info(dat.0, dat.1).await;
                                    tx.send(Message::AlbumInfoRetrieved(album))
                                        .await
                                        .expect("send")
                                },
                            ))
                                .map(cosmic::Action::App)
                        }
                        _ => {
                            // should never happen
                            log::error!("Requested album info while outside albums page somehow")
                        }
                    }
                }
                app::Message::GridSliderChange(val) => {
                    self.config
                        .set_grid_item_size(&self.config_handler, val)
                        .expect("Failed To Update Config");
                }
                app::Message::SeekTrack(val) => {
                    self.sink.set_volume(0.0);
                    log::info!("volume: {}", self.sink.volume());
                    match self.sink.try_seek(Duration::from_secs_f64(val)) {
                        Ok(_) => {}
                        Err(_) => {}
                    }
                }
                Message::SeekFinished => {
                    self.sink.set_volume(1.0)
                }
                app::Message::AddTrackToQueue(filepath) => {
                    let pos = self.nav.entity_at(0).expect("REASON");
                    let home_page = self.nav.data_mut::<Page>(pos).unwrap();
                    if let Page::NowPlaying(page) = home_page {
                        let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();
                        let mut stmt = conn
                            .prepare(
                                "
                                select track.name as title, art.name as artist, track.path, a.album_cover, a.name as album_title
                                from track
                                left join main.album_tracks at on track.id = at.track_id
                                left join main.artists art on track.artist_id = art.id
                                left join main.album a on at.album_id = a.id
                                where track.path=?;
                            ",
                            )
                            .expect("error preparing sql");

                        let track = stmt
                            .query_row([&filepath], |row| {
                                Ok(AppTrack {
                                    title: row.get("title").expect("REASON"),
                                    artist: row.get("artist").expect("REASON"),
                                    album_title: row.get("album_title").expect("REASON"),
                                    path_buf: PathBuf::from(
                                        row.get::<&str, String>("path").expect("REASON"),
                                    ),
                                    cover_art: match row.get::<&str, Vec<u8>>("album_cover") {
                                        Ok(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
                                        Err(_) => None,
                                    },
                                })
                            })
                            .expect("error executing query");

                        self.queue.push(track);
                    }

                    match self.sink.empty() {
                        true => {
                            let file = std::fs::File::open(filepath).expect("Failed to open file");

                            let decoder = rodio::Decoder::builder()
                                .with_byte_len(file.metadata().unwrap().len())
                                .with_data(file)
                                .with_seekable(true)
                                .build()
                                .expect("Failed to build decoder");

                            self.song_duration = match decoder.total_duration() {
                                None => {
                                    log::error!("Failed to decode song duration");
                                    None
                                }
                                Some(val) => {
                                    log::info!("Decoded song duration: {}", val.as_secs_f64());
                                    Some(val.as_secs_f64())
                                }
                            };
                            self.sink.append(decoder);
                            let sleeping_task_sink = Arc::clone(&self.sink);
                            let sleeping_thread = cosmic::task::future(async move {
                                let kill = true;
                                Message::SongFinished(
                                    tokio::task::spawn_blocking(move || {
                                        if kill {
                                            sleeping_task_sink.sleep_until_end();
                                            QueueUpdateReason::None
                                        } else {
                                            QueueUpdateReason::ThreadKilled
                                        }
                                    })
                                        .await
                                        .expect("cosmic_music.db"),
                                )
                            }).abortable();

                            match &mut self.task_handle {
                                None => {
                                    self.task_handle = Some(vec![sleeping_thread.1]);
                                }
                                Some(handles) => {
                                    handles.push(sleeping_thread.1);
                                }
                            }

                            let reporting_task_sink = Arc::clone(&self.sink);
                            let progress_thread = cosmic::Task::stream(
                                cosmic::iced_futures::stream::channel(10, |mut tx| async move {
                                    tokio::task::spawn_blocking(move || loop {
                                        sleep(Duration::from_millis(10));
                                        match tx.try_send(Message::SinkProgress(
                                            reporting_task_sink.get_pos().as_secs_f64(),
                                        )) {
                                            Ok(_) => {}
                                            Err(_) => {break}
                                        }
                                    });
                                }),
                            ).abortable();

                            match &mut self.task_handle {
                                None => {
                                    self.task_handle = Some(vec![progress_thread.1])
                                }
                                Some(handles) => {
                                    handles.push(progress_thread.1)
                                }
                            }
                            let (task, handle) = cosmic::task::batch(vec![progress_thread.0, sleeping_thread.0]).abortable();
                            match &mut self.task_handle {
                                None => {
                                    self.task_handle = Some(vec![handle])
                                }
                                Some(handles) => {
                                    handles.push(handle)
                                }
                            }
                            self.sink.play();
                            return task;
                        }
                        false => {}
                    }
                }
                Message::SinkProgress(number) => {
                    self.song_progress = number;
                }
                Message::SongFinished(val) => {
                    let sink = self.sink.clone();

                    if self.queue.is_empty() {
                        self.queue_pos = 0;
                        self.song_progress = 0.0;
                        self.song_duration = None;
                        return cosmic::Task::none()
                    }

                    match val {
                        QueueUpdateReason::Skipped => {
                            if self.queue_pos + 1 > self.queue.len() - 1 {
                                self.queue_pos = 0;
                            } else {
                                self.queue_pos += 1;
                            }

                            self.clear = true;
                            sink.clear();
                            sink.play()
                        }
                        QueueUpdateReason::Previous => {
                            if self.queue_pos as i32 - 1 < 0 {
                                self.queue_pos = self.queue.len() - 1;
                            } else {
                                self.queue_pos -= 1;
                            }
                            self.clear = true;
                            sink.clear();
                            sink.play()
                        }
                        QueueUpdateReason::None => {
                            match self.clear {
                                true => {
                                    self.clear = false;
                                    match self.queue.is_empty() {
                                        true => {}
                                        false => {
                                            let file = self.queue.get(self.queue_pos).unwrap().path_buf.clone().to_string_lossy().to_string();
                                            return cosmic::task::future(async move {
                                                Message::AddTrackToSink(file)
                                            });
                                        }
                                    }
                                }
                                false => {
                                    return match self.loop_state {
                                        LoopState::LoopingTrack => {
                                            let file = self.queue.get(self.queue_pos).unwrap().path_buf.clone().to_string_lossy().to_string();
                                            cosmic::task::future(async move {
                                                Message::AddTrackToSink(file)
                                            })
                                        }
                                        LoopState::LoopingQueue => {
                                            if self.queue_pos + 1 > self.queue.len() - 1 {
                                                self.queue_pos = 0;
                                            } else {
                                                self.queue_pos += 1;
                                            }
                                            sink.play();
                                            let file = self.queue.get(self.queue_pos).unwrap().path_buf.clone().to_string_lossy().to_string();
                                            cosmic::task::future(async move {
                                                Message::AddTrackToSink(file)
                                            })
                                        }
                                        LoopState::NotLooping => {
                                            if self.queue_pos + 1 > self.queue.len() - 1 {
                                                self.queue_pos = 0;
                                                sink.pause()
                                            } else {
                                                self.queue_pos += 1;
                                            }

                                            let file = self.queue.get(self.queue_pos).unwrap().path_buf.clone().to_string_lossy().to_string();
                                            cosmic::task::future(async move {
                                                Message::AddTrackToSink(file)
                                            })
                                        }
                                    }
                                }
                            }
                        }
                        QueueUpdateReason::Removed(index) => {
                            if self.queue_pos > index {
                                self.queue_pos -= 1;

                                self.clear = true;
                                self.sink.clear();
                                self.sink.play();
                                return cosmic::Task::none();
                            }

                            if index as i32 == (self.queue.len() as i32 - 1) && self.queue_pos as i32 == (self.queue.len() as i32) - 1 {
                                self.queue_pos = 0;
                                self.queue.remove(index);


                                self.clear = true;
                                self.sink.clear();
                                if let LoopState::LoopingQueue = self.loop_state {
                                    self.sink.play();
                                }
                                return cosmic::Task::none();
                            } else {
                                self.queue.remove(index);
                                self.clear = true;
                                self.sink.clear();
                                self.sink.play();
                            }



                        }
                        QueueUpdateReason::ThreadKilled => {}
                    }
                }
                Message::AddTrackToSink(filepath) => {
                    let file = std::fs::File::open(filepath).expect("Failed to open file");

                    let decoder = rodio::Decoder::builder()
                        .with_byte_len(file.metadata().unwrap().len())
                        .with_data(file)
                        .with_seekable(true)
                        .build()
                        .expect("Failed to build decoder");

                    self.song_duration = Some(decoder.total_duration().unwrap().as_secs_f64());
                    self.sink.append(decoder);

                    let task_sink = Arc::clone(&self.sink);
                    let (task, handle) = cosmic::task::future(async move {
                        Message::SongFinished(
                            tokio::task::spawn_blocking(move || {
                                task_sink.sleep_until_end();
                                QueueUpdateReason::None
                            })
                                .await
                                .expect("cosmic_music.db"),
                        )
                    }).abortable();

                    match &mut self.task_handle {
                        None => {
                            self.task_handle = Some(vec![handle])
                        }
                        Some(handles) => {
                            handles.push(handle);
                        }
                    }

                    return task;
                }
                Message::SkipTrack => {
                    return cosmic::task::future(async move {
                        Message::SongFinished(QueueUpdateReason::Skipped)
                    });
                }
                Message::ClearQueue => {
                    self.clear = true;
                    self.sink.stop();
                    match &self.task_handle {
                        None => {}
                        Some(handles) => {
                            for handle in handles {
                                handle.abort()
                            }
                        }
                    }

                    self.queue_pos = 0;
                    self.song_progress = 0.0;
                    self.song_duration = None;

                    self.queue.clear();
                }
                Message::PreviousTrack => {
                    return cosmic::task::future(async move {
                        Message::SongFinished(QueueUpdateReason::Previous)
                    });
                }
                app::Message::AddAlbumToQueue(paths) => {
                    return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                        0,
                        |mut tx| async move {
                            for file in paths {

                                tx.send(Message::AddTrackToQueue(file)).await.expect("send")
                            }
                        },
                    ))
                        .map(cosmic::Action::App)
                }

                Message::PlayPause => match self.sink.is_paused() {
                    true => {
                        self.sink.play();
                    }
                    false => {
                        self.sink.pause();
                    }
                },
                Message::CreatePlaylist => {
                    todo!()
                }
            };

            Task::none()
        }

        /// Called when a nav item is selected.
        fn on_nav_select(&mut self, id: nav_bar::Id) -> Task<cosmic::Action<Self::Message>> {
            // Activate the page in the model.
            self.nav.activate(id);
            self.update_title()
        }
    }

    impl AppModel {
        /// The about page for this app.
        pub fn about(&self) -> Element<Message> {
            let cosmic_theme::Spacing { space_xxs, .. } = theme::active().cosmic().spacing;

            let icon = widget::svg(widget::svg::Handle::from_memory(APP_ICON));

            let title = widget::text::title3(fl!("app-title"));

            let hash = env!("VERGEN_GIT_SHA");
            let short_hash: String = hash.chars().take(7).collect();
            let date = env!("VERGEN_GIT_COMMIT_DATE");

            let link = widget::button::link(REPOSITORY)
                .on_press(Message::OpenRepositoryUrl)
                .padding(0);

            widget::column()
                .push(icon)
                .push(title)
                .push(link)
                .push(
                    widget::button::link(fl!(
                    "git-description",
                    hash = short_hash.as_str(),
                    date = date
                ))
                        .on_press(Message::LaunchUrl(format!("{REPOSITORY}/commits/{hash}")))
                        .padding(0),
                )
                .align_x(Alignment::Center)
                .spacing(space_xxs)
                .into()
        }

        /// Updates the header and window titles.
        pub fn update_title(&mut self) -> Task<cosmic::Action<Message>> {
            let mut window_title = fl!("app-title");

            if let Some(page) = self.nav.text(self.nav.active()) {
                window_title.push_str(" — ");
                window_title.push_str(page);
            }

            fn do_thing() -> Task<cosmic::Action<Message>> {
                return cosmic::task::future(async move { Message::OnNavEnter });
            }
            ;

            if let Some(id) = self.core.main_window_id() {
                return cosmic::Task::batch(vec![self.set_window_title(window_title, id), do_thing()]);
            } else {
                Task::none()
            }
        }
    }

    /// The page to display in the application.
    #[derive(Debug)]
    pub enum Page {
        NowPlaying(HomePage),
        Artists,
        Albums(AlbumPage),
        Playlists,
        Tracks(TrackPage),
    }

    /// The context page to display in the context drawer.
    #[derive(Copy, Clone, Debug, Default, Eq, PartialEq)]
    pub enum ContextPage {
        #[default]
        About,
        Settings,
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub enum Action {
        About,
        Settings,
        UpdateScanProgress(u32),
        UpdateScanDirSize(u32),
    }

    impl menu::action::MenuAction for Action {
        type Message = Message;

        fn message(&self) -> Self::Message {
            match self {
                Action::About => Message::ToggleContextPage(ContextPage::About),
                Action::Settings => Message::ToggleContextPage(ContextPage::Settings),
                Action::UpdateScanProgress(num) => Message::UpdateScanProgress(*num),
                Action::UpdateScanDirSize(num) => Message::UpdateScanDirSize(*num),
            }
        }
    }
