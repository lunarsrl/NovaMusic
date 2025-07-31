// SPDX-License-Identifier: GPL-2.0-or-later

use cosmic::dialog::file_chooser::{self, Error, FileFilter};
use cosmic::widget::dropdown::multi::model;
use rayon::iter::IndexedParallelIterator;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
mod albums;
mod artists;
pub(crate) mod home;
mod playlists;
mod scan;
mod search;
mod settings;
mod tracks;

use std::any::TypeId;
use tokio::task::{spawn_blocking, JoinHandle};

use crate::app::albums::{
    get_album_info, get_top_album_info, Album, AlbumPage, AlbumPageState, FullAlbum,
};

use crate::app::home::{listify_queue, HomePage};
use crate::app::scan::{scan_directory, MediaFileTypes};
use crate::app::tracks::{SearchResult, TrackPage, TrackPageState};

use crate::app::Message::{
    AddTrackToSink, AlbumPageStateAlbum, AlbumProcessed, AlbumRequested, SearchResults,
    SongFinished, UpdateScanProgress,
};
use crate::config::Config;
use crate::database::{create_database, create_database_entry};
use crate::{app, config, fl};
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
use cosmic::widget::{
    self, container, dialog, icon, menu, nav_bar, progress_bar, toaster, JustifyContent,
};
use cosmic::{action, cosmic_theme, iced, iced_futures, theme};
use futures::channel::mpsc;
use futures::channel::mpsc::{Receiver, SendError, Sender, TrySendError};
use futures_util::stream::{Next, SelectNextSome};
use futures_util::{SinkExt, StreamExt};
use log::info;
use rodio::source::SeekError::SymphoniaDecoder;

use crate::app::playlists::{FullPlaylist, Playlist, PlaylistPage, PlaylistPageState, PlaylistTrack};
use cosmic::cctk::wayland_protocols::ext::session_lock::v1::client::ext_session_lock_manager_v1::ExtSessionLockManagerV1;
use cosmic::dialog::file_chooser::open::FileResponse;
use cosmic::iced::task::Handle;
use cosmic::iced::Alignment::Start;

use rayon::iter::{IntoParallelIterator, ParallelBridge};
use rayon::slice::ParallelSliceMut;
use regex::Match;
use rodio::source::SeekError;
use rodio::{OutputStream, Sink, Source};
use rusqlite::fallible_iterator::FallibleIterator;
use std::collections::HashMap;
use std::fmt::{format, Debug};
use std::{fs, io};
use std::fs::File;
use std::future::Future;
use std::io::{BufRead, BufReader, Write};
use std::ops::DerefMut;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use std::thread::sleep;
use std::time::Duration;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_AAC, CODEC_TYPE_NULL};
use symphonia::core::formats::{FormatOptions, Track};
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
    /// Dialog
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

    pub playlist_dialog: bool,

    // Searches
    pub search_field: String,
    pub playlist_dialog_text: String,
    pub playlist_cover: Vec<u8>,
    footer_toggled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AppTrack {
    pub id: u32,
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub path_buf: PathBuf,
    pub cover_art: Option<cosmic::widget::image::Handle>,
}

/// Messages emitted by the application and its widgets.

#[derive(Debug)]
pub enum LoopState {
    LoopingTrack,
    LoopingQueue,
    NotLooping,
}
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    UpdateConfig(Config),
    LaunchUrl(String),
    EditInput(String),

    // Config change related
    RescanDir,

    // Filesystem scan related
    ChangeScanDir(String),
    UpdateScanProgress((u32, u32, u32)),
    UpdateScanDirSize(u32),

    // Page Rendering
    OnNavEnter,

    // Album Page
    AlbumRequested((String, String)), // when an album icon is clicked [gets title & artist of album]
    AlbumInfoRetrieved(FullAlbum), // when task assigned to retrieving requested albums info is completed [gets full track list of album]
    AlbumProcessed(Vec<Album>), // when an album retrieved from db's data is organized and ready [Supplies AlbumPage with the new Album]
    AlbumsLoaded, // when albums table retrieved from db is exhausted after OnNavEnter in Album Page [Sets page state to loaded]
    AlbumPageStateAlbum(AlbumPage), // when album info is retrieved [Replaces AlbumPage with AlbumPage with new info] todo: Might be able to use this weird implementation to cache one album visit
    AlbumPageReturn,

    // Home Page
    //todo Change to pathbufs for safety?
    AddTrackToQueue(String),
    //todo Make albums in queue fancier kinda like Elisa does it
    AddAlbumToQueue(Vec<String>),

    // Track Page
    TracksLoaded,
    TrackLoaded(Vec<AppTrack>),
    UpdateSearch(String),
    SearchResults(Vec<crate::app::tracks::SearchResult>),
    ToggleTitle(bool),
    ToggleAlbum(bool),
    ToggleArtist(bool),

    // Audio Messages
    PlayPause,
    SongFinished(QueueUpdateReason),
    AddTrackToSink(String),
    SkipTrack,
    ChangeLoopState,
    PreviousTrack,
    SeekFinished,
    ClearQueue,
    SinkProgress(f64),
    SeekTrack(f64),
    ChangeActiveInQueue(usize),
    RemoveSongInQueue(usize),

    // Settings
    ChooseFolder,
    FolderChosen(String),
    FolderPickerFail,
    GridSliderChange(u32),
    VolumeSliderChange(f32),

    //experimenting
    AddToPlaylist,
    CreatePlaylist,
    UpdatePlaylistName(String),

    FooterToggle,
    PlaylistFound(Vec<Playlist>),
    PlaylistSelected(Playlist),
    PlaylistPageReturn,
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
    const APP_ID: &'static str = "dev.riveroluna.novamusic";

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
        // store data & first time set up
        match dirs::data_local_dir()
            .unwrap()
            .join(crate::app::AppModel::APP_ID)
            .is_dir()
        {
            true => {}
            false => fs::create_dir(
                dirs::data_local_dir()
                    .unwrap()
                    .join(crate::app::AppModel::APP_ID),
            )
            .unwrap(),
        }

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
            .data::<Page>(Page::Tracks(TrackPage::new()))
            .icon(icon::from_name("media-tape-symbolic"));

        nav.insert()
            .text(fl!("albums"))
            .data::<Page>(Page::Albums(AlbumPage::new(vec![])))
            .icon(icon::from_name("media-optical-symbolic"));

        // todo Add playlist support
        nav.insert()
            .text(fl!("playlists"))
            .data::<Page>(Page::Playlists(PlaylistPage::new()))
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


        sink.set_volume(config.volume / 100.0);
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
            playlist_dialog: false,
            search_field: "".to_string(),
            playlist_dialog_text: "".to_string(),
            playlist_cover: vec![],
            footer_toggled: true,
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
    fn footer(&self) -> Option<Element<Self::Message>> {
        if !self.footer_toggled {
            return Some(
                cosmic::widget::container(
                    cosmic::widget::row::with_children(vec![
                        cosmic::widget::horizontal_space().into(),
                        cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                            "go-down-symbolic",
                        ))
                        .on_press(Message::FooterToggle)
                        .into(),
                    ])
                    .padding(cosmic::theme::spacing().space_xxs),
                )
                .into(),
            );
        }

        let mut time_elapsed = crate::app::home::format_time(self.song_progress);

        let mut total_duration = "**:**".to_string();
        match self.song_duration {
            None => {}
            Some(val) => {
                total_duration = crate::app::home::format_time(val);
            }
        };

        const FOOTER_IMAGE_SIZE: f32 = 64.0;
        let data = match self.queue.is_empty() {
            true => {
                let cover = cosmic::widget::icon::from_name("applications-audio-symbolic")
                    .size(FOOTER_IMAGE_SIZE as u16)
                    .into();

                (None, None, None, cover)
            }
            false => {
                let title = Some(self.queue.get(self.queue_pos).unwrap().title.as_str());
                let artist = Some(self.queue.get(self.queue_pos).unwrap().artist.as_str());
                let album = Some(self.queue.get(self.queue_pos).unwrap().album_title.as_str());

                let cover = match &self.queue.get(self.queue_pos).unwrap().cover_art {
                    None => cosmic::widget::icon::from_name("media-playback-start-symbolic")
                        .size(FOOTER_IMAGE_SIZE as u16)
                        .into(),
                    Some(val) => cosmic::widget::image(val)
                        .width(Length::Fixed(FOOTER_IMAGE_SIZE))
                        .height(Length::Fixed(FOOTER_IMAGE_SIZE))
                        .content_fit(ContentFit::ScaleDown)
                        .into(),
                };

                (title, artist, album, cover)
            }
        };

        let play_pause_button: cosmic::Element<Message> = match self.queue.is_empty() {
            true => {
                self.sink.clear();
                cosmic::widget::button::icon(match self.sink.is_paused() {
                    true => cosmic::widget::icon::from_name("media-playback-start-symbolic"),
                    false => cosmic::widget::icon::from_name("media-playback-pause-symbolic"),
                })
                .into()
            }
            false => cosmic::widget::button::icon(match self.sink.is_paused() {
                true => cosmic::widget::icon::from_name("media-playback-start-symbolic"),
                false => cosmic::widget::icon::from_name("media-playback-pause-symbolic"),
            })
            .on_press(Message::PlayPause)
            .into(),
        };

        return Some(
            cosmic::widget::container(
                cosmic::widget::container(
                    cosmic::widget::row::with_children(vec![
                        data.3,
                        // Media Progress
                        cosmic::widget::column::with_children(vec![
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading(data.0.unwrap_or("None")).into(),
                                cosmic::widget::text::heading(data.1.unwrap_or("None")).into(),
                                cosmic::widget::text::heading(data.2.unwrap_or("")).into(),
                                cosmic::widget::horizontal_space().into(),
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "go-up-symbolic",
                                ))
                                .on_press(Message::FooterToggle)
                                .into(),
                            ])
                            .spacing(cosmic::theme::spacing().space_s)
                            .into(),
                            cosmic::widget::row::with_children(vec![
                                cosmic::widget::text::heading(time_elapsed).into(),
                                cosmic::widget::slider(
                                    0.0..=self.song_duration.unwrap_or(1.0),
                                    self.song_progress,
                                    |a| Message::SeekTrack(a),
                                )
                                .on_release(Message::SeekFinished)
                                .height(31.0)
                                .into(),
                                cosmic::widget::text::heading(format!("{}", total_duration)).into(),
                                // Media Controls
                                cosmic::widget::row::with_children(vec![
                                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                        "media-skip-backward-symbolic",
                                    ))
                                    .on_press(Message::PreviousTrack)
                                    .into(),
                                    // PLAY OR PAUSE
                                    play_pause_button,
                                    // PLAY OR PAUSE
                                    cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                        "media-skip-forward-symbolic",
                                    ))
                                    .on_press(Message::SkipTrack)
                                    .into(),
                                    cosmic::widget::button::icon(match self.loop_state {
                                        LoopState::LoopingTrack => cosmic::widget::icon::from_name(
                                            "media-playlist-repeat-song-symbolic",
                                        ),
                                        LoopState::LoopingQueue => cosmic::widget::icon::from_name(
                                            "media-playlist-no-repeat-symbolic",
                                        ),
                                        LoopState::NotLooping => cosmic::widget::icon::from_name(
                                            "media-playlist-consecutive-symbolic",
                                        ),
                                    })
                                    .on_press(Message::ChangeLoopState)
                                    .into(),
                                ])
                                .width(Length::Shrink)
                                .align_y(Vertical::Center)
                                .spacing(cosmic::theme::spacing().space_xxxs)
                                .into(),
                            ])
                            .width(Length::Fill)
                            .align_y(Vertical::Center)
                            .spacing(cosmic::theme::spacing().space_xxs)
                            .into(),
                        ])
                        .into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_xs),
                )
                .width(Length::Fill)
                .padding(cosmic::theme::spacing().space_xxs)
                .class(cosmic::theme::Container::Primary),
            )
            .align_y(Start)
            .width(Length::Fill)
            .into(),
        );
    }

    /// Enables the COSMIC application to create a nav bar with this model.
    fn nav_model(&self) -> Option<&nav_bar::Model> {
        Some(&self.nav)
    }

    fn dialog(&self) -> Option<Element<Self::Message>> {
        if !dirs::data_local_dir()
            .unwrap()
            .join(crate::app::AppModel::APP_ID)
            .join("nova_music.db")
            .exists()
        {
            match dirs::data_local_dir()
                .unwrap()
                .join(crate::app::AppModel::APP_ID)
                .is_dir()
            {
                true => {}
                false => fs::create_dir(
                    dirs::data_local_dir()
                        .unwrap()
                        .join(crate::app::AppModel::APP_ID),
                )
                .unwrap(),
            }

            return Some(
                cosmic::widget::dialog::Dialog::new()
                    .title("Nova Music: First Time Setup")
                    .body("Please choose a directory to scan")
                    .control(cosmic::widget::container(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(format!(
                                "Current Directory: {}",
                                self.config.scan_dir.as_str()
                            ))
                            .into(),
                            cosmic::widget::horizontal_space().into(),
                            cosmic::widget::button::text("Select Folder")
                                .on_press(Message::ChooseFolder)
                                .into(),
                        ]),
                    ))
                    .primary_action(
                        cosmic::widget::button::text("Scan Folder & Create Database")
                            .class(cosmic::theme::Button::Suggested)
                            .on_press(Message::RescanDir),
                    )
                    .into(),
            );
        }

        if self.playlist_dialog {
            return Some(
                cosmic::widget::dialog::Dialog::new()
                    .title("Create A New Playlist")
                    .icon(icon::from_name("applications-audio-symbolic"))
                    .control(cosmic::widget::container(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(format!("Playlist Title: ")).into(),
                            cosmic::widget::horizontal_space().into(),
                            cosmic::widget::text_input(
                                "Enter Title",
                                self.playlist_dialog_text.as_str(),
                            )
                            .on_input(|input| Message::UpdatePlaylistName(input))
                            .into(),
                        ]),
                    ))
                    .primary_action(
                        cosmic::widget::button::text("Create Playlist")
                            .class(cosmic::theme::Button::Suggested)
                            .on_press(Message::CreatePlaylist),
                    )
                    .into(),
            );
        }
        None
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
            Page::NowPlaying(home_page) => home_page.load_page(&self),
            Page::Tracks(track_page) => track_page.load_page(self),
            Page::Albums(album_page) => album_page.load_page(self),
            Page::Playlists(playlist_page) => playlist_page.load_page(self),
        }
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::UpdatePlaylistName(val) => self.playlist_dialog_text = val,
            Message::ChooseFolder => {
                return cosmic::task::future(async move {
                    let dialog = cosmic::dialog::file_chooser::open::Dialog::new();
                    match dialog.open_folder().await {
                        Ok(selected) => {
                            let fp = selected
                                .url()
                                .to_owned()
                                .to_file_path()
                                .unwrap()
                                .to_str()
                                .unwrap()
                                .to_string();
                            Message::FolderChosen(fp)
                        }
                        Err(err) => {
                            // todo toasts for file picking errors
                            log::info!("folder picker fail: {}", err);
                            Message::FolderPickerFail
                        }
                    }
                })
                .map(action::Action::App);
            }
            Message::FolderPickerFail => {
                self.config
                    .set_scan_dir(&self.config_handler, "".to_string());
            }
            Message::FolderChosen(fp) => {
                self.rescan_available = true;
                self.config.set_scan_dir(&self.config_handler, fp);
            }
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
                    Err(err) => {}
                },
                Err(error) => {}
            },

            Message::UpdateConfig(config) => {
                self.config = config;
            }
            Message::UpdateSearch(search) => {
                self.search_field = search;

                let regex = match regex::RegexBuilder::new(self.search_field.as_str())
                    .case_insensitive(true)
                    .build()
                {
                    Ok(a) => a,
                    Err(err) => {
                        panic!("{}", err)
                    }
                };

                match self.nav.active_data::<Page>().unwrap() {
                    Page::NowPlaying(_) => {}
                    Page::Albums(page) => {
                        let cloned_albums = page.albums.clone();

                        return cosmic::Task::stream(
                            cosmic::iced_futures::stream::channel(0, |mut tx| async move {
                                tokio::task::spawn_blocking(move || {
                                    let mut albums = cloned_albums
                                        .par_iter()
                                        .enumerate()
                                        .map(|(index, album)| {
                                            return match regex.find(&album.name) {
                                                None => SearchResult {
                                                    tracks_index: index,
                                                    score: 999,
                                                },
                                                Some(val) => {
                                                    if val.range().start == 0 {
                                                        if val.range().end == album.name.len() {
                                                            // Exact Match
                                                            return SearchResult {
                                                                tracks_index: index,
                                                                score: 0,
                                                            };
                                                        }
                                                        // Matches at the beginning

                                                        return SearchResult {
                                                            tracks_index: index,
                                                            score: 1,
                                                        };
                                                    }
                                                    // Matches somewhere else
                                                    SearchResult {
                                                        tracks_index: index,
                                                        score: 2,
                                                    }
                                                }
                                            };
                                        })
                                        .collect::<Vec<SearchResult>>();

                                    albums.sort_by(|a, b| a.score.cmp(&b.score));
                                    tx.try_send(Message::SearchResults(albums))
                                });
                                ()
                            })
                            .map(action::Action::App),
                        );
                    }
                    Page::Playlists(_) => {}
                    Page::Tracks(page) => {
                        let cloned_tracks = page.tracks.clone();

                        return cosmic::Task::stream(
                            cosmic::iced_futures::stream::channel(0, |mut tx| async move {
                                tokio::task::spawn_blocking(move || {
                                    let mut tracks = cloned_tracks
                                        .par_iter()
                                        .enumerate()
                                        .map(|(index, track)| {
                                            match regex.find(&track.title) {
                                                None => {
                                                    match regex.find(&track.album_title) {
                                                        None => {
                                                            match regex.find(&track.artist) {
                                                                None => SearchResult {
                                                                    tracks_index: index,
                                                                    score: 999,
                                                                },
                                                                Some(val) => {
                                                                    if val.range().start == 0 {
                                                                        if val.range().end
                                                                            == track.artist.len()
                                                                        {
                                                                            // Exact Match
                                                                            return SearchResult {
                                                                                tracks_index: index,
                                                                                score: 6,
                                                                            };
                                                                        }
                                                                        // Matches at the beginning

                                                                        return SearchResult {
                                                                            tracks_index: index,
                                                                            score: 7,
                                                                        };
                                                                    }
                                                                    // Matches somewhere else
                                                                    return SearchResult {
                                                                        tracks_index: index,
                                                                        score: 8,
                                                                    };
                                                                }
                                                            }
                                                        }
                                                        Some(val) => {
                                                            if val.range().start == 0 {
                                                                if val.range().end
                                                                    == track.album_title.len()
                                                                {
                                                                    // Exact Match
                                                                    return SearchResult {
                                                                        tracks_index: index,
                                                                        score: 3,
                                                                    };
                                                                }
                                                                // Matches at the beginning

                                                                return SearchResult {
                                                                    tracks_index: index,
                                                                    score: 4,
                                                                };
                                                            }
                                                            // Matches somewhere else
                                                            return SearchResult {
                                                                tracks_index: index,
                                                                score: 5,
                                                            };
                                                        }
                                                    }
                                                }
                                                Some(val) => {
                                                    if val.range().start == 0 {
                                                        if val.range().end == track.title.len() {
                                                            // Exact Match
                                                            return SearchResult {
                                                                tracks_index: index,
                                                                score: 0,
                                                            };
                                                        }
                                                        // Matches at the beginning

                                                        return SearchResult {
                                                            tracks_index: index,
                                                            score: 1,
                                                        };
                                                    }
                                                    // Matches somewhere else
                                                    return SearchResult {
                                                        tracks_index: index,
                                                        score: 2,
                                                    };
                                                }
                                            }
                                        })
                                        .collect::<Vec<SearchResult>>();

                                    tracks.sort_by(|a, b| a.score.cmp(&b.score));
                                    tx.try_send(Message::SearchResults(tracks))
                                });
                                ()
                            })
                            .map(action::Action::App),
                        );
                    }
                }
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
                self.config.set_tracks_found(&self.config_handler, 0);
                self.config.set_albums_found(&self.config_handler, 0);

                // Albums: Full reset
                let album_pos = self.nav.entity_at(2).unwrap();
                let album_dat = self.nav.data_mut::<Page>(album_pos).unwrap();

                if let Page::Albums(page) = album_dat {
                    page.albums = Arc::from(vec![]);
                    page.page_state = AlbumPageState::Loading
                }

                // Tracks: Full reset
                let track_pos = self.nav.entity_at(1).unwrap();
                let track_dat = self.nav.data_mut::<Page>(track_pos).unwrap();
                if let Page::Tracks(page) = track_dat {
                    page.track_page_state = TrackPageState::Loading
                }

                create_database();

                let path = self.config.scan_dir.clone().parse().unwrap();
                return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                    10,
                    |mut tx| async move {
                        let files = scan_directory(path, &mut tx).await;
                        let mut files_scanned = 0;

                        for file in files {
                            files_scanned += 1;
                            match file {
                                MediaFileTypes::FLAC(path) => {
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
                                MediaFileTypes::MP4(path) => {
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
                                MediaFileTypes::MP3(path) => {
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
                                            log::warn!("Something went wrong: {}", err);
                                            continue;
                                        }
                                    };

                                    // ID3v1 metadata is stored at the end while ID3v2 is stored at the beginning. I don't know how to fix that.
                                    if let Some(mdat_revision) = reader.metadata.get() {
                                        if let Some(mdat_revision) = mdat_revision.current() {
                                            let metadata_tags = mdat_revision
                                                .tags()
                                                .iter()
                                                .filter(|a| a.is_known())
                                                .map(|a| a.clone())
                                                .collect::<Vec<Tag>>();
                                            create_database_entry(metadata_tags, &path)
                                        } else {
                                            log::warn!("no metadata found")
                                        }
                                    } else {
                                    }
                                }
                            }
                        }

                        let conn = rusqlite::Connection::open(
                            dirs::data_local_dir()
                                .unwrap()
                                .join(Self::APP_ID)
                                .join("nova_music.db"),
                        )
                        .unwrap();

                        let albums_found =
                            conn.query_row("select count(*) from album", [], |row| {
                                Ok(row.get::<usize, u32>(0).unwrap_or(0))
                            });

                        let tracks_found =
                            conn.query_row("select count(*) from track", [], |row| {
                                Ok(row.get::<usize, u32>(0).unwrap_or(0))
                            });

                        tx.send(Message::UpdateScanProgress((
                            albums_found.unwrap_or(0),
                            tracks_found.unwrap_or(0),
                            files_scanned,
                        )))
                        .await
                        .expect("Hello");

                        tx.send(Message::OnNavEnter).await.expect("de")
                    },
                ))
                // Must wrap our app type in `cosmic::Action`.
                .map(cosmic::Action::App);
            }
            Message::UpdateScanProgress((album, tracks, files)) => {
                self.config
                    .set_albums_found(&self.config_handler, album)
                    .expect("Failed to save to config");
                self.config
                    .set_tracks_found(&self.config_handler, tracks)
                    .expect("Failed to save to config");
                self.config
                    .set_files_scanned(&self.config_handler, files)
                    .expect("Failed to save to config");
                self.rescan_available = true;
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
                    dat.albums = Arc::from(new_album)
                }
            }
            Message::OnNavEnter => {
                self.search_field = "".to_string();

                match self.nav.active_data_mut().unwrap() {
                    Page::NowPlaying(HomePage) => {}
                    Page::Albums(val) => {
                        match &mut val.page_state {
                            AlbumPageState::Loading => {
                                log::info!("LOADING");
                            }
                            AlbumPageState::Album(page) => {
                                log::info!("ALBUM PAGE");
                                return Task::none();
                            }
                            AlbumPageState::Loaded => {
                                log::info!("LOADED");
                                return Task::none();
                            }
                            AlbumPageState::Search(results) => {
                                log::info!("SEARCHING STATE");
                                return Task::none();
                            }
                        }

                        return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                            5,
                            |mut tx| async move {
                                tokio::task::spawn_blocking(move || {
                                    let conn =
                                        match rusqlite::Connection::open(
                                            dirs::data_local_dir().unwrap().join(Self::APP_ID).join("nova_music.db")
                                        ) {
                                            Ok(conn) => conn,
                                            Err(err) => {
                                                panic!("{}", err)
                                            }
                                        };


                                    let mut stmt = match conn
                                        .prepare(
                                            "
                                            SELECT a.id, a.name,
                                            a.disc_number, a.track_number, a.album_cover, art.name as artist_name
                                            FROM album a
                                            left JOIN artists art ON a.artist_id = art.id",
                                        ) {
                                        Ok(stmt) => stmt,
                                        Err(err) => {
                                            panic!("{}", err)
                                        }
                                    };

                                    let album_iter = stmt
                                        .query_map([], |row| {
                                            Ok((
                                                row.get::<_, String>("name").unwrap_or("None".to_string()),
                                                row.get::<_, String>("artist_name").unwrap_or_default(),
                                                row.get::<_, u32>("disc_number").unwrap_or(0),
                                                row.get::<_, u32>("track_number").unwrap_or(0),
                                                match row.get::<_, Vec<u8>>("album_cover") {
                                                    Ok(val) => Some(val),
                                                    Err(e) => {
                                                        log::info!("Nothing");
                                                        None
                                                    }
                                                },
                                            ))
                                        })
                                        .expect("error executing query");


                                    let albums: Vec<(String, String, u32, u32, Option<Vec<u8>>)> =
                                        album_iter.filter_map(|a| {
                                            a.ok()
                                        }).collect();
                                    for each in &albums {
                                        log::info!("ALBUM: {:?}", each.0);
                                    }
                                    get_top_album_info(&mut tx, albums);

                                    tx.try_send(Message::AlbumsLoaded)
                                });
                            },
                        ))
                            .map(cosmic::Action::App);
                    }
                    Page::Playlists(page) => match page.playlist_page_state {
                        PlaylistPageState::Loading => {
                            match dirs::data_local_dir()
                                .unwrap()
                                .join(crate::app::AppModel::APP_ID)
                                .join("Playlists")
                                .is_dir()
                            {
                                true => {}
                                false => fs::create_dir(
                                    dirs::data_local_dir()
                                        .unwrap()
                                        .join(crate::app::AppModel::APP_ID)
                                        .join("Playlists"),
                                )
                                .unwrap(),
                            }

                            return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                                5,
                                |mut tx| async move {
                                    tokio::task::spawn_blocking(move || {
                                        let dir = fs::read_dir(
                                            dirs::data_local_dir()
                                                .unwrap()
                                                .join(crate::app::AppModel::APP_ID)
                                                .join("Playlists"),
                                        )
                                        .expect("yo");

                                        let mut playlists = vec![];

                                        for file in dir {
                                            if let Ok(file) = file {

                                                let files = io::BufReader::new(fs::File::open(file.path()).unwrap());
                                                let path = file.path();
                                                let mut is_m3u = false;

                                                for (index, line) in files.lines().filter_map(Result::ok).enumerate() {
                                                    let mut title = String::from("");
                                                    let mut cover = String::from("");

                                                    if !is_m3u {
                                                    if line.contains("#EXTM3U") && index == 0 {
                                                        is_m3u = true;
                                                        log::info!("is m3u")
                                                    } else {
                                                        continue
                                                    }
                                                    }

                                                    if is_m3u && line.contains("#PLAYLIST:") {
                                                        title = line.replace("#PLAYLIST:", "");

                                                        log::info!("title: {}", title);
                                                        let playlist = Playlist {
                                                            title,
                                                            path: path.to_str().unwrap().to_string(),
                                                            thumbnail: None,
                                                        } ;

                                                        playlists.push(playlist);
                                                        break;
                                                    } else {

                                                    }
                                                }
                                            }
                                        }
                                        tx.try_send(Message::PlaylistFound(playlists)).expect("send error");
                                    });
                                },
                            ))
                            .map(cosmic::Action::App);
                        }
                        PlaylistPageState::Loaded => {}
                        PlaylistPageState::PlaylistPage(_) => {}
                    },
                    Page::Tracks(page) => match page.track_page_state {
                        TrackPageState::Loading => {
                            return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                                100,
                                |mut tx| async move {
                                    tokio::task::spawn_blocking(move || {
                                        let conn = rusqlite::Connection::open(
                                            dirs::data_local_dir()
                                                .unwrap()
                                                .join(Self::APP_ID)
                                                .join("nova_music.db"),
                                        )
                                        .unwrap();
                                        let mut stmt = conn.prepare(
                                            "
select track.id as id, track.name as title, art.name as artist, track.path, a.name as album_title
from track
         left join main.album_tracks at on track.id = at.track_id
         left join main.artists art on track.artist_id = art.id
         left join main.album a on at.album_id = a.id;
                        ",
                                        );

                                        if let Ok(mut stmt) = stmt {
                                            let tracks = stmt
                                                .query_map([], |row| {
                                                    Ok(AppTrack {
                                                        id: row.get("id").unwrap_or(0),
                                                        title: row
                                                            .get("title")
                                                            .unwrap_or("No Data".to_string()),
                                                        artist: row
                                                            .get("artist")
                                                            .unwrap_or("N/A".to_string()),
                                                        album_title: row
                                                            .get("album_title")
                                                            .unwrap_or("N/A".to_string()),
                                                        path_buf: PathBuf::from(
                                                            row.get::<&str, String>("path")
                                                                .expect("This should never happen"),
                                                        ),
                                                        cover_art: None,
                                                    })
                                                })
                                                .expect("error executing query");

                                            let tracks =
                                                tracks.into_iter().filter_map(|a| a.ok()).collect();

                                            tx.try_send(Message::TrackLoaded(tracks))
                                                .expect("Failed to send");
                                            tx.try_send(Message::TracksLoaded)
                                                .expect("Failed to send");
                                        } else {
                                            tx.try_send(Message::TracksLoaded)
                                                .expect("Failed to send");
                                        }
                                    });
                                },
                            ))
                            .map(cosmic::Action::App);
                        }
                        TrackPageState::Loaded => {}
                        TrackPageState::Search => {
                            page.track_page_state = TrackPageState::Loaded;
                        }
                    },
                }
            }

            Message::PlaylistFound(playlists) => {
                if let Page::Playlists(page) = self.access_nav_data(3) {
                    page.playlists = Arc::new(playlists);
                    log::info!("Playlists: {:?}", page);
                    page.playlist_page_state = PlaylistPageState::Loaded;
                }
            }
            Message::PlaylistSelected(playlist) => {
                if let Page::Playlists(page) = self.access_nav_data(3) {

                    let mut tracks = vec![];

                    let files = io::BufReader::new(
                        match fs::File::open(&playlist.path) {
                            Ok(val) => {val}
                            Err(err) => {
                                log::info!("{}", err);
                                return cosmic::task::none();
                            }
                        }
                    );
                    let mut is_m3u = false;
                    let mut track_title = None;



                    for (index, line) in files.lines().filter_map(Result::ok).enumerate() {
                        let mut title = String::from("");
                        let mut cover = String::from("");

                        if !is_m3u {
                            if line.contains("#EXTM3U") && index == 0 {
                                is_m3u = true;
                                log::info!("is m3u")
                            } else {
                                continue
                            }
                        }

                        if is_m3u && line.contains("#EXTINF:0,") {
                            let line = line.replace("#EXTINF:0,", "");
                            let title_divide = line.find(" - ").unwrap();
                            let line = line[title_divide + 2..].to_string();

                            track_title = Some(line);
                            continue
                        } else {

                        }

                        if is_m3u && track_title.is_some() {
                            let path =  PathBuf::from(line);

                            tracks.push(PlaylistTrack {
                                title: track_title.take().unwrap().parse().unwrap(),
                                path: path.to_string_lossy().to_string(),
                            });
                        }
                    }



                    let lpay = FullPlaylist {
                        playlist,
                        tracks
                    };
                    page.playlist_page_state = PlaylistPageState::PlaylistPage(lpay);
                }
            }
            Message::PlaylistPageReturn => {
                if let Page::Playlists(page) = self.access_nav_data(3) {
                    page.playlist_page_state = PlaylistPageState::Loaded;
                }
            }
            Message::TrackLoaded(mut track) => {
                let dat_pos = self.nav.entity_at(1).expect("REASON");
                let data = self.nav.data_mut::<Page>(dat_pos).unwrap();

                if let Page::Tracks(dat) = data {
                    dat.tracks = Arc::new(track)
                }
            }
            Message::TracksLoaded => {
                let dat_pos = self.nav.entity_at(1).expect("REASON");
                let data = self.nav.data_mut::<Page>(dat_pos).unwrap();

                if let Page::Tracks(dat) = data {
                    dat.track_page_state = TrackPageState::Loaded;
                }
            }
            Message::AlbumsLoaded => {
                let dat_pos = self.nav.entity_at(2).expect("REASON");
                let data = self.nav.data_mut::<Page>(dat_pos).unwrap();

                if let Page::Albums(dat) = data {
                    dat.page_state = AlbumPageState::Loaded;
                    dat.has_fully_loaded = true;
                    log::info!("{:?}", dat.albums);
                    log::info!("EMPTY ALBUMS");
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

                    Page::Albums(old_page) => {
                        *old_page = new_page;
                    }
                    Page::Playlists(page) => {}
                    Page::Tracks(page) => {}
                }
            }
            Message::AlbumInfoRetrieved(albuminfopage) => {
                log::info!("ALBUM INFO RETRIEVED: {:?}", albuminfopage);
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
                match self.sink.try_seek(Duration::from_secs_f64(val)) {
                    Ok(_) => {}
                    Err(_) => {}
                }
            }
            Message::SeekFinished => self.sink.set_volume(self.config.volume/100.0),
            app::Message::AddTrackToQueue(filepath) => {
                let pos = self.nav.entity_at(0).expect("REASON");
                let home_page = self.nav.data_mut::<Page>(pos).unwrap();
                if let Page::NowPlaying(page) = home_page {
                    let conn = rusqlite::Connection::open(
                        dirs::data_local_dir()
                            .unwrap()
                            .join(Self::APP_ID)
                            .join("nova_music.db"),
                    )
                    .unwrap();
                    let mut stmt = conn
                        .prepare(
                            "
                                select track.id as id, track.name as title, art.name as artist, track.path, a.album_cover, a.name as album_title
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
                                id: row.get("id").unwrap_or(0),
                                title: row.get("title").unwrap_or("".to_string()),
                                artist: row.get("artist").unwrap_or("".to_string()),
                                album_title: row.get("album_title").unwrap_or("".to_string()),
                                path_buf: PathBuf::from(
                                    row.get::<&str, String>("path")
                                        .expect("There should always be a file path"),
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
                            None => None,
                            Some(val) => Some(val.as_secs_f64()),
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
                                .expect("nova_music.db"),
                            )
                        })
                        .abortable();

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
                            cosmic::iced_futures::stream::channel(1, |mut tx| async move {
                                tokio::task::spawn_blocking(move || loop {
                                    sleep(Duration::from_millis(200));
                                    match tx.try_send(Message::SinkProgress(
                                        reporting_task_sink.get_pos().as_secs_f64(),
                                    )) {
                                        Ok(_) => {}
                                        Err(_) => break,
                                    }
                                });
                            }),
                        )
                        .abortable();

                        match &mut self.task_handle {
                            None => self.task_handle = Some(vec![progress_thread.1]),
                            Some(handles) => handles.push(progress_thread.1),
                        }
                        let (task, handle) =
                            cosmic::task::batch(vec![progress_thread.0, sleeping_thread.0])
                                .abortable();
                        match &mut self.task_handle {
                            None => self.task_handle = Some(vec![handle]),
                            Some(handles) => handles.push(handle),
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
                log::info!(
                    "Song finished: {:?} | {} | {:?}",
                    val,
                    self.clear,
                    self.loop_state
                );
                let sink = self.sink.clone();

                if self.queue.is_empty() {
                    self.queue_pos = 0;
                    self.song_progress = 0.0;
                    self.song_duration = None;
                    self.sink.clear();
                    return cosmic::Task::none();
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
                    QueueUpdateReason::None => match self.clear {
                        true => {
                            self.clear = false;
                            match self.queue.is_empty() {
                                true => {}
                                false => {
                                    let file = self
                                        .queue
                                        .get(self.queue_pos)
                                        .unwrap()
                                        .path_buf
                                        .clone()
                                        .to_string_lossy()
                                        .to_string();
                                    return cosmic::task::future(async move {
                                        Message::AddTrackToSink(file)
                                    });
                                }
                            }
                        }
                        false => {
                            return match self.loop_state {
                                LoopState::LoopingTrack => {
                                    let file = self
                                        .queue
                                        .get(self.queue_pos)
                                        .unwrap()
                                        .path_buf
                                        .clone()
                                        .to_string_lossy()
                                        .to_string();
                                    cosmic::task::future(
                                        async move { Message::AddTrackToSink(file) },
                                    )
                                }
                                LoopState::LoopingQueue => {
                                    if self.queue_pos + 1 > self.queue.len() - 1 {
                                        self.queue_pos = 0;
                                    } else {
                                        self.queue_pos += 1;
                                    }
                                    sink.play();
                                    let file = self
                                        .queue
                                        .get(self.queue_pos)
                                        .unwrap()
                                        .path_buf
                                        .clone()
                                        .to_string_lossy()
                                        .to_string();
                                    cosmic::task::future(
                                        async move { Message::AddTrackToSink(file) },
                                    )
                                }
                                LoopState::NotLooping => {
                                    if self.queue_pos + 1 > self.queue.len() - 1 {
                                        self.queue_pos = 0;
                                        sink.pause()
                                    } else {
                                        self.queue_pos += 1;
                                    }

                                    let file = self
                                        .queue
                                        .get(self.queue_pos)
                                        .unwrap()
                                        .path_buf
                                        .clone()
                                        .to_string_lossy()
                                        .to_string();
                                    cosmic::task::future(
                                        async move { Message::AddTrackToSink(file) },
                                    )
                                }
                            }
                        }
                    },
                    QueueUpdateReason::Removed(index) => {
                        if self.queue_pos > index {
                            self.queue_pos -= 1;

                            self.queue.remove(index);

                            return cosmic::Task::none();
                        }

                        if index as i32 == (self.queue.len() as i32 - 1)
                            && self.queue_pos as i32 == (self.queue.len() as i32) - 1
                        {
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
                            if index == self.queue_pos {
                                self.clear = true;
                                self.sink.clear();
                                self.sink.play();
                            }
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
                        .expect("nova_music.db"),
                    )
                })
                .abortable();

                match &mut self.task_handle {
                    None => self.task_handle = Some(vec![handle]),
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
            Message::AddToPlaylist => self.playlist_dialog = true,
            Message::CreatePlaylist => {
                match dirs::data_local_dir()
                    .unwrap()
                    .join(crate::app::AppModel::APP_ID)
                    .join("Playlists")
                    .is_dir()
                {
                    true => {}
                    false => fs::create_dir(
                        dirs::data_local_dir()
                            .unwrap()
                            .join(crate::app::AppModel::APP_ID)
                            .join("Playlists"),
                    )
                    .unwrap(),
                }
                let dir_path = dirs::data_local_dir()
                    .unwrap()
                    .join(crate::app::AppModel::APP_ID)
                    .join("Playlists");
                let mut new_file =
                    fs::File::create(&dir_path.join(format!("{}.m3u", &self.playlist_dialog_text)))
                        .expect("Failed to create Playlist file");
                new_file
                    .write(
                        format!("#EXTM3U \n#PLAYLIST:{}\n", self.playlist_dialog_text).as_bytes(),
                    )
                    .expect("Failed to write Playlist file");
                for track in &self.queue {
                    new_file
                        .write(
                            format!(
                                "#EXTINF:0,{} - {}\n{}\n",
                                track.artist,
                                track.title,
                                track.path_buf.to_string_lossy().to_string()
                            )
                            .as_bytes(),
                        )
                        .expect("Failed to write Playlist file");
                }

                self.playlist_dialog = false;
                if let Page::Playlists(page) = self.access_nav_data(3) {
                    page.playlist_page_state = PlaylistPageState::Loading
                }
            }
            Message::SearchResults(tracks) => {
                match self
                    .nav
                    .active_data_mut::<Page>()
                    .expect("Pages should exist always")
                {
                    Page::NowPlaying(_) => {}

                    Page::Albums(page) => page.page_state = AlbumPageState::Search(tracks),
                    Page::Playlists(page) => {}
                    Page::Tracks(track_list) => {
                        track_list.track_page_state = TrackPageState::Search;
                        track_list.search = tracks;
                    }
                }
            }
            Message::ToggleTitle(val) => {
                if let Page::Tracks(page) = self.access_nav_data(1) {
                    page.search_by_title = val
                }
            }
            Message::ToggleAlbum(val) => {
                if let Page::Tracks(page) = self.access_nav_data(1) {
                    page.search_by_album = val
                }
            }
            Message::ToggleArtist(val) => {
                if let Page::Tracks(page) = self.access_nav_data(1) {
                    page.search_by_artist = val
                }
            }
            Message::VolumeSliderChange(val) => {
                log::info!("volume: {}", val);
                self.sink.set_volume(val / 100.0);
                self.config
                    .set_volume(&self.config_handler, val)
                    .expect("Failed to set volume");
            }
            Message::FooterToggle => {
                if let Page::Albums(page) = self.access_nav_data(2) {
                    log::info!("page:  {:?}", page.albums)
                } else {
                    log::info!("When did this happen")
                }

                match self.footer_toggled {
                    true => self.footer_toggled = false,
                    false => self.footer_toggled = true,
                }
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
    fn access_nav_data(&mut self, pos: u16) -> &mut Page {
        let dat_pos = self.nav.entity_at(pos).expect("REASON");
        let data = self.nav.data_mut::<Page>(dat_pos).unwrap();
        data
    }
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
        };

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

    Albums(AlbumPage),
    Playlists(PlaylistPage),
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
    UpdateScanProgress((u32, u32, u32)),
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
