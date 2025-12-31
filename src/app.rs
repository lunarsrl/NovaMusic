// SPDX-License-Identifier: GPL-2.0-or-later

use crate::mpris::player::MPRISPlayer;
use cosmic::dialog::file_chooser::Error;
use regex::Regex;

use rayon::iter::IndexedParallelIterator;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};

pub(crate) mod home;
mod page;

mod scan;
mod settings;

use crate::app::home::HomePage;
use crate::app::page::albums::{Album, AlbumPage, AlbumPageState, FullAlbum};
use crate::app::page::artists::{ArtistInfo, ArtistPage, ArtistPageState, ArtistsPage};
use crate::app::page::genre::{GenrePage, GenrePageState};
use crate::app::page::playlists::{
    FullPlaylist, Playlist, PlaylistPage, PlaylistPageState, PlaylistTrack,
};
use crate::app::page::tracks::{SearchResult, TrackPage, TrackPageState};
use crate::app::page::CoverArt;
use crate::app::page::CoverArt::SomeLoaded;
use crate::app::scan::scan_directory;
use crate::config::{AppTheme, Config, SortBy};
use crate::database::{create_database, create_database_entry, find_visual};
use crate::mpris::MPRISRootInterface;
use crate::{app, config, fl};
use colored::Colorize;
use cosmic::app::context_drawer;
use cosmic::cosmic_theme::palette::cam16::Cam16IntoUnclamped;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::keyboard::key;
use cosmic::iced::task::Handle;
use cosmic::iced::window::Id;
use cosmic::iced::Alignment::Start;
use cosmic::iced::{keyboard, Alignment, Color, ContentFit, Event, Length};
use cosmic::iced_widget::list;
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::prelude::*;
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::Action::App;
use cosmic::{action, cosmic_config, cosmic_theme, theme};
use event_listener::Listener;
use futures_util::{SinkExt, StreamExt, TryStreamExt};
use rodio::{Sink, Source};
use rusqlite::fallible_iterator::FallibleIterator;
use std::any::TypeId;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, Read, Write as OtherWrite};
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};
use symphonia::default::get_probe;
use zbus::export::ordered_stream::OrderedStreamExt;
use zbus::{connection, Connection, MatchRule, MessageStream};

const REPOSITORY: &str = env!("CARGO_PKG_REPOSITORY");
const APP_ICON: &[u8] =
    include_bytes!("../resources/icons/hicolor/scalable/apps/dev.lunarsrl.NovaMusic.svg");

/// The application model stores app-specific state used to describe its interface and
/// drive its logic.
pub struct AppModel {
    /// Application state which is managed by the COSMIC runtime.
    core: cosmic::Core,

    // MPRIS
    pub connection: Option<zbus::Connection>,

    // Navigation
    context_page: ContextPage,
    nav: nav_bar::Model,
    key_binds: HashMap<menu::KeyBind, Action>,

    // Config
    config: Config,
    config_handler: cosmic_config::Config,

    //Settings Page
    pub rescan_available: bool,

    //Audio
    pub mixer: rodio::stream::OutputStream,
    pub sink: Arc<Sink>,
    pub loop_state: LoopState,
    pub song_progress: f64,
    pub song_duration: Option<f64>,
    pub queue: Vec<AppTrack>,
    pub queue_pos: usize,
    pub clear: bool,
    pub task_handle: Option<Vec<Handle>>,

    // dialogs
    pub playlist_creation_dialog: bool,
    pub artistpage_edit_dialog: bool,
    pub artistspage_edit_dialog: bool,

    // Searches
    pub search_active: bool,
    pub search_field: String,
    pub playlist_dialog_text: String,
    playlist_dialog_path: String,
    pub playlist_cover: Option<PathBuf>,
    playlist_delete_dialog: bool,
    playlist_edit_dialog: bool,

    // Error Handling
    toasts: cosmic::widget::toaster::Toasts<Message>,

    // Navigation
    albumsid: nav_bar::Id,
    tracksid: nav_bar::Id,
    artistsid: nav_bar::Id,
    playlistsid: nav_bar::Id,
    homeid: nav_bar::Id,
    genreid: nav_bar::Id,
    search_id: cosmic::iced_core::id::Id,
}

#[derive(Clone, Debug, PartialEq, Eq)]
/* todo: This is wasteful with memory at the benefit of less database accesses
I think that the cost of accessing the database is much less important than the cost
of having every track in the queue, for example, which is only displayed on one page,
be in the global state of the application as an AppTrack. Only the first and second tracks up
next in the queue should be AppTracks. The rest can just be ids that are turned into AppTracks
as they approach. This should save a lot of memory
*/
/// All info associated with a track
pub struct AppTrack {
    pub id: u32,
    pub title: String,
    pub artist: String,
    pub album_title: String,
    pub path_buf: PathBuf,
    pub cover_art: crate::app::page::CoverArt,
}

/// Minimum amount of info required to display fully expose a Single track
#[derive(Debug, Clone)]
pub struct DisplaySingle {
    pub id: u32,
    pub title: String,
    pub artist: String,
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
pub enum FileChooserEvents {
    ArtistPagePicture,
}

#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    UpdateTheme(AppTheme),
    LaunchUrl(String),

    // MPRIS

    // Config change related
    RescanDir,
    // For people who dont have an xdg file chooser :)
    ManualScanDirEdit(String),

    // Filesystem related
    ChooseFolder,
    FolderChosen(String),
    FolderPickerFail(String),
    UpdateScanProgress,
    UpdateScanDirSize,
    AddToDatabase(PathBuf),
    ProbeFail,
    ChooseFile(FileChooserEvents),

    // Page Rendering
    OnNavEnter(ReEnterNavReason),
    ScrollView(Viewport),

    // Album Page
    AlbumProcessed(Vec<Album>), // when an album retrieved from db's data is organized and ready [Supplies AlbumPage with the new Album]
    AlbumsLoaded, // when albums table retrieved from db is exhausted after OnNavEnter in Album Page [Sets page state to loaded]
    AlbumPageStateAlbum(AlbumPage), // when album info is retrieved [Replaces AlbumPage with AlbumPage with new info]
    AlbumPageReturn,

    // impl for Artists & Album Page
    AlbumRequested((String, String)), // when an album icon is clicked [gets title & artist of album]
    AlbumInfoRetrieved(FullAlbum), // when task assigned to retrieving requested albums info is completed [gets full track list of album]

    // Home Page
    //todo move all AddTrackToQueue to AddTrackByID
    // Advantage: No need to clone strings
    // Disadvantage: Database access but that happens anyway sometimes
    AddTrackToQueue(String),
    AddTrackById((TrackType, u32)),
    //todo Make albums in queue fancier kinda like Elisa does it
    AddAlbumToQueue(Vec<(String, u32)>),

    // Track Page
    TracksLoaded,
    TrackLoaded(Vec<AppTrack>),
    ToggleTitle(bool),
    ToggleAlbum(bool),
    ToggleArtist(bool),

    // Artists Page
    ArtistsLoaded(Vec<ArtistInfo>),
    ArtistsPageEdit,
    ArtistRequested(String),
    //Artist
    ArtistPageReturn,
    // Dialog Toggles
    ArtistPageEdit,

    // Playlist Page
    AddToPlaylist,
    CreatePlaylistConfirm,
    UpdatePlaylistName(String),
    PlaylistFound(Vec<Playlist>),
    PlaylistSelected(Playlist),
    PlaylistPageReturn,
    PlaylistDeleteSafety,
    PlaylistDeleteConfirmed,

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
    GridSliderChange(u32),
    VolumeSliderChange(f32),

    // Footer
    ToggleFooter(bool),

    // Error Reporting
    Toasts(cosmic::widget::toaster::ToastId),
    ToastError(String),

    //experimenting
    /// Dialogs
    CreatePlaylistCancel,
    CreatePlaylistAddThumbnail,
    CreatePlaylistIconChosen(PathBuf),
    PlaylistEdit(String),
    EditPlaylistConfirm,
    EditPlaylistCancel,
    EditArtistConfirm,
    ArtistAddPicture(String),

    // Menu Bar
    Sort(SortBy),
    SearchActivate,
    SearchClear,
    SearchInput(String),
    SearchResults,
    PageDataRecieved(Vec<AppTrack>),
}

#[derive(Clone, Debug)]
pub enum ReEnterNavReason {
    UserInteraction,
    Rescan,
    ArtistEdit,
    PlaylistEdit,
}

#[derive(Clone, Debug)]
pub enum QueueUpdateReason {
    Skipped,
    Previous,
    Removed(usize),
    None,
    ThreadKilled,
}

#[derive(Debug, Clone)]
pub enum TrackType {
    AlbumTrack,
    Single,
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
    const APP_ID: &'static str = "dev.lunarsrl.NovaMusic";

    fn core(&self) -> &cosmic::Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut cosmic::Core {
        &mut self.core
    }

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
        let mixer =
            rodio::OutputStreamBuilder::open_default_stream().expect("Failed to open stream");
        let sink = rodio::Sink::connect_new(mixer.mixer());

        let sink = Arc::new(sink);

        let homeid = nav
            .insert()
            .text(fl!("home"))
            .data::<Page>(Page::NowPlaying(HomePage { viewport: None }))
            .icon(icon::from_name("applications-audio-symbolic"))
            .activate()
            .id();

        let tracksid = nav
            .insert()
            .text(fl!("tracks"))
            .data::<Page>(Page::Tracks(TrackPage::new()))
            .icon(icon::from_name("media-tape-symbolic"))
            .id();

        let artistsid = nav
            .insert()
            .text(fl!("artists"))
            .data::<Page>(Page::Artist(ArtistsPage::new()))
            .icon(icon::from_name("avatar-default-symbolic"))
            .id();

        let albumsid = nav
            .insert()
            .text(fl!("albums"))
            .data::<Page>(Page::Albums(AlbumPage::new()))
            .icon(icon::from_name("media-optical-symbolic"))
            .id();

        let playlistsid = nav
            .insert()
            .text(fl!("playlists"))
            .data::<Page>(Page::Playlists(PlaylistPage::new()))
            .icon(icon::from_name("playlist-symbolic"))
            .id();

        let genreid = nav
            .insert()
            .text(fl!("genres"))
            .data::<Page>(Page::Genre(GenrePage::new()))
            .icon(icon::from_name("playlist-symbolic"))
            .id();

        // INIT CONFIG
        let config = config::Config::load();
        let config_handler = match config.0 {
            None => {
                panic!("No config exists");
            }
            Some(som) => som,
        };

        // init toasts
        sink.set_volume(config.1.volume / 100.0);
        // Construct the app model with the runtime's core.
        let mut app = AppModel {
            core,
            connection: None,

            //Navigation:
            context_page: ContextPage::default(),
            nav,
            key_binds: HashMap::new(),

            // Optional configuration file for an application.
            config: config.1,
            config_handler,
            rescan_available: true,
            // Audio
            mixer,
            sink,
            loop_state: LoopState::NotLooping,
            song_progress: 0.0,
            song_duration: None,
            queue: vec![],
            queue_pos: 0,
            clear: false,
            task_handle: None,

            // Search related
            search_field: "".to_string(),
            search_active: false,

            // dialogs toggles

            //playlist
            playlist_creation_dialog: false,
            playlist_delete_dialog: false,
            playlist_edit_dialog: false,

            //artist
            artistpage_edit_dialog: false,
            artistspage_edit_dialog: false,

            // playlist dialog additional data
            playlist_dialog_text: "".to_string(),
            playlist_dialog_path: "".to_string(),
            playlist_cover: None,

            // footer
            toasts: cosmic::widget::toaster::Toasts::new(|a| Message::Toasts(a)),
            albumsid,
            tracksid,
            artistsid,
            playlistsid,
            homeid,
            genreid,
            search_id: cosmic::iced_core::id::Id::unique(),
        };

        // Start up commands

        let commands = cosmic::Task::batch(vec![
            app.update_title(),
            // cosmic::Task::future(async { Message::MPRISCheck }).map(action::app),
        ]);
        (app, commands)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![
            menu::Tree::with_children(
                menu::root(fl!("view")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::Button(fl!("settings"), None, Action::Settings),
                        menu::Item::Button(fl!("about"), None, Action::About),
                    ],
                ),
            ),
            menu::Tree::with_children(
                menu::root(fl!("sort")).apply(Element::from),
                menu::items(
                    &self.key_binds,
                    vec![
                        menu::Item::CheckBox(
                            "A-Z",
                            None,
                            self.config.sort_option == SortBy::AscendingName,
                            Action::SortChange(SortBy::AscendingName),
                        ),
                        menu::Item::CheckBox(
                            "Z-A",
                            None,
                            self.config.sort_option == SortBy::DescendingName,
                            Action::SortChange(SortBy::DescendingName),
                        ),
                    ],
                ),
            ),
        ]);
        vec![menu_bar.into()]
    }

    fn header_end(&self) -> Vec<Element<'_, Self::Message>> {
        let search = match self.search_active {
            true => widget::text_input::search_input("", &self.search_field)
                .width(Length::Fixed(240.0))
                .id(self.search_id.clone())
                .on_clear(Message::SearchClear)
                .on_input(Message::SearchInput)
                .into(),
            false => widget::button::icon(widget::icon::from_name("system-search-symbolic"))
                .on_press(Message::SearchActivate)
                .padding(8)
                .into(),
        };

        vec![search]
    }

    fn on_close_requested(&self, _id: Id) -> Option<Self::Message> {
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
        if !self.config.footer {
            return None;
        }

        let time_elapsed = crate::app::home::format_time(self.song_progress);

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
                    _ => cosmic::widget::icon::from_name("media-playback-start-symbolic")
                        .size(FOOTER_IMAGE_SIZE as u16)
                        .into(),
                    SomeLoaded(val) => cosmic::widget::image(val)
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
                                cosmic::widget::text::heading(data.0.unwrap_or("")).into(),
                                cosmic::widget::text::heading(data.1.unwrap_or("")).into(),
                                cosmic::widget::text::heading(data.2.unwrap_or("")).into(),
                                cosmic::widget::horizontal_space().into(), // todo Context menu for mini player options
                                                                           // cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                                                           //     "go-up-symbolic",
                                                                           // ))
                                                                           // .on_press(Message::ToggleFooter)
                                                                           // .into(),
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
                    .title(fl!("firsttimetitle"))
                    .body(fl!("firsttimebody"))
                    .control(cosmic::widget::container(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text_input(
                                fl!("pathtofolder"),
                                self.config.scan_dir.as_str(),
                            )
                            .on_input(|val| Message::ManualScanDirEdit(val))
                            .into(),
                            cosmic::widget::horizontal_space().into(),
                            cosmic::widget::button::text(fl!("folderselect"))
                                .class(cosmic::theme::style::Button::Standard)
                                .on_press(Message::ChooseFolder)
                                .into(),
                        ]),
                    ))
                    .primary_action(
                        cosmic::widget::button::text(fl!("firsttimeprimary"))
                            .class(cosmic::theme::Button::Suggested)
                            .on_press(Message::RescanDir),
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
        let body;
        match self.nav.active_data::<Page>().unwrap() {
            Page::NowPlaying(home_page) => body = home_page.load_page(self),
            Page::Tracks(track_page) => body = track_page.load_page(self),
            Page::Artist(artists_page) => body = artists_page.load_page(self),
            Page::Albums(album_page) => body = album_page.load_page(self),
            Page::Playlists(playlist_page) => body = playlist_page.load_page(self),
            Page::Genre(genre_page) => body = genre_page.load_page(self),
        }

        cosmic::widget::container(cosmic::widget::column::with_children(vec![
            cosmic::widget::toaster(&self.toasts, cosmic::widget::horizontal_space()).into(),
            body,
        ]))
        .into()
    }

    /// Handles messages emitted by the application and its widgets.
    ///
    /// Tasks may be returned for asynchronous execution of code in the background
    /// on the application's async runtime.
    fn update(&mut self, message: Self::Message) -> Task<cosmic::Action<Self::Message>> {
        match message {
            Message::SearchInput(val) => self.search_field = val,
            Message::SearchClear => {
                self.search_active = false;
                self.search_field = String::from("");
            }
            Message::SearchActivate => {
                self.search_active = true;
                return cosmic::widget::text_input::focus(self.search_id.clone());
            }
            Message::Sort(val) => {
                if val != self.config.sort_option {
                    self.config
                        .set_sort_option(&self.config_handler, val)
                        .expect("Config change failed");
                }
            }
            Message::UpdateTheme(selection) => {
                self.config.set_app_theme(&self.config_handler, selection);
                return cosmic::command::set_theme(self.config.app_theme.theme());
            }
            Message::EditArtistConfirm => {
                if let Page::Artist(artist) = self.nav.active_data_mut::<Page>().unwrap() {
                    if let ArtistPageState::ArtistPage(ref page) = artist.page_state {
                        let conn = connect_to_db();

                        if let Ok(result) = conn.query_row(
                            "select id from artists where name = ?",
                            [&page.artist.name],
                            |row| row.get::<_, u32>("id"),
                        ) {
                            let file =
                                fs::File::open(PathBuf::from(page.artist.path.clone())).unwrap();
                            if let Ok(result) = conn.execute(
                                "update artists set artistpfp = ? where id = ?",
                                (
                                    &Box::new(
                                        file.bytes().filter_map(|a| a.ok()).collect::<Vec<u8>>(),
                                    ),
                                    &result,
                                ),
                            ) {
                                log::info!("{} entries changed in artists table!", result);
                                return cosmic::Task::future(
                                    async move { Message::ArtistPageEdit },
                                )
                                .map(cosmic::action::Action::App);
                            } else {
                                log::warn!("artists table could not be updated")
                            }
                        } else {
                            log::warn!("Artist: {} not found", page.artist.name)
                        }
                    }
                }
            }
            Message::ManualScanDirEdit(val) => {
                self.config.set_scan_dir(&self.config_handler, val).unwrap();
            }
            Message::ToastError(error) => {
                return self
                    .toasts
                    .push(cosmic::widget::toaster::Toast::new(error))
                    .map(cosmic::Action::App);
            }
            Message::Toasts(id) => self.toasts.remove(id),
            Message::ScrollView(view) => match self
                .nav
                .active_data_mut::<Page>()
                .expect("Should always be intialized")
            {
                Page::NowPlaying(page) => page.viewport = Some(view),
                Page::Albums(page) => {
                    page.viewport = Some(view);
                }
                Page::Playlists(page) => page.viewport = Some(view),
                Page::Tracks(page) => page.viewport = Some(view),
                Page::Artist(page) => page.viewport = Some(view),
                Page::Genre(page) => page.viewport = Some(view),
            },
            Message::PlaylistDeleteConfirmed => {
                if let Page::Playlists(toppage) =
                    self.nav.data_mut::<Page>(self.playlistsid).unwrap()
                {
                    if let PlaylistPageState::PlaylistPage(page) = &toppage.playlist_page_state {
                        match std::fs::remove_file(&page.playlist.path) {
                            Ok(_) => {
                                toppage.playlist_page_state = PlaylistPageState::Loading;
                                match self.playlist_delete_dialog {
                                    true => self.playlist_delete_dialog = false,
                                    false => self.playlist_delete_dialog = true,
                                }

                                return cosmic::Task::future(async move {
                                    Message::OnNavEnter(ReEnterNavReason::PlaylistEdit)
                                })
                                .map(cosmic::Action::App);
                            }
                            Err(err) => {
                                log::error!("Failed to delete file \n -------- \n Err: {}", err);

                                return cosmic::task::future(async move {
                                    Message::ToastError(String::from("File could not be removed!"))
                                });
                            }
                        }
                    }
                }
                log::error!("Event triggered in the wrong state");
                self.playlist_delete_dialog = false;
            }
            Message::PlaylistEdit(path) => {
                let string = String::from("");
                let mut cover_path: String = String::from("");

                if string.contains("#EXTALBUMARTURL:") {
                    for each in string.lines() {
                        if each.contains("#EXTALBUMARTURL:") {
                            cover_path = each.replace("#EXTALBUMARTURL:", "").to_string();
                        }
                    }

                    self.playlist_cover = Some(PathBuf::from(cover_path))
                } else {
                    self.playlist_cover = None;
                }

                self.playlist_dialog_path = path;
                match self.playlist_edit_dialog {
                    true => self.playlist_edit_dialog = false,
                    false => self.playlist_edit_dialog = true,
                }
            }

            Message::ArtistPageEdit => match self.artistpage_edit_dialog {
                true => self.artistpage_edit_dialog = false,
                false => self.artistpage_edit_dialog = true,
            },

            Message::ArtistsPageEdit => match self.artistspage_edit_dialog {
                true => self.artistspage_edit_dialog = false,
                false => self.artistspage_edit_dialog = true,
            },
            Message::PlaylistDeleteSafety => match self.playlist_delete_dialog {
                true => self.playlist_delete_dialog = false,
                false => self.playlist_delete_dialog = true,
            },
            Message::UpdatePlaylistName(val) => self.playlist_dialog_text = val,
            Message::ChooseFile(message) => {
                return cosmic::Task::future(async move {
                    let dialog = cosmic::dialog::file_chooser::open::Dialog::new();
                    let file = dialog.open_file().await;

                    match file {
                        Ok(fr) => {
                            let path = fr.0.uris().get(0).unwrap().path().to_string();
                            match message {
                                FileChooserEvents::ArtistPagePicture => {
                                    Message::ArtistAddPicture(path)
                                }
                            }
                        }
                        Err(err) => {
                            log::error!("{}", err);
                            Message::ToastError(String::from("Something went wrong..."))
                        }
                    }
                })
                .map(action::Action::App)
            }
            Message::ArtistAddPicture(path) => {
                if let Page::Artist(toppage) = self.nav.active_data_mut::<Page>().unwrap() {
                    if let ArtistPageState::ArtistPage(ref mut page) = toppage.page_state {
                        if let Ok(file) = fs::File::open(PathBuf::from(path.clone())) {
                            let handle = cosmic::widget::image::Handle::from_bytes(
                                file.bytes().filter_map(|a| a.ok()).collect::<Vec<u8>>(),
                            );
                            page.artist.image = Some(handle);
                            page.artist.path = path;
                        } else {
                            return cosmic::Task::future(async move {
                                Message::ToastError(String::from("Could not load image!"))
                            })
                            .map(cosmic::action::Action::App);
                        }
                    } else {
                        panic!("This event is being used in thr wrong state!")
                    }
                } else {
                    panic!("This event is being used in thr wrong state!")
                }
            }
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
                            let error: String;
                            match err {
                                Error::Cancelled => {
                                    error = String::from(
                                        "Cancelled File Picker: Keeping scan directory the same",
                                    )
                                }
                                Error::Close(err) => {
                                    error = String::from(format!("Closer: {}", err.to_string()))
                                }
                                Error::Open(err) => {
                                    error = String::from(format!("Open: {}", err.to_string()))
                                }
                                Error::Response(err) => {
                                    error = String::from(format!("Response: {}", err.to_string()))
                                }
                                Error::Save(err) => {
                                    error = String::from(format!("Save: {}", err.to_string()))
                                }
                                Error::SetDirectory(err) => {
                                    error =
                                        String::from(format!("Set Directory: {}", err.to_string()))
                                }
                                Error::SetAbsolutePath(err) => {
                                    error = String::from(format!(
                                        "Set Absolute Path: {}",
                                        err.to_string()
                                    ))
                                }
                                Error::UrlAbsolute => error = String::from("URL Absolute"),
                            }
                            Message::FolderPickerFail(error)
                        }
                    }
                })
                .map(action::Action::App);
            }
            Message::FolderPickerFail(error) => {
                if !(error.contains("Cancelled File Picker: Keeping scan directory the same")) {
                    let _ = self
                        .config
                        .set_scan_dir(&self.config_handler, "".to_string());
                } else {
                }

                return self
                    .toasts
                    .push(cosmic::widget::toaster::Toast::new(error))
                    .map(cosmic::Action::App);
            }
            Message::FolderChosen(fp) => {
                self.rescan_available = true;
                let _ = self.config.set_scan_dir(&self.config_handler, fp);
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

                if fs::exists(self.config.scan_dir.as_str()).is_err() {
                    return cosmic::Task::future(async move {
                        Message::ToastError(fl!("ScanFileDoesNotExist"))
                    })
                    .map(cosmic::Action::App);
                }

                self.queue_pos = 0;
                self.song_progress = 0.0;
                self.song_duration = None;

                self.queue.clear();

                // Settings: No rescan until current rescan finishes
                self.rescan_available = false;

                log::info!("{}", self.rescan_available);
                self.config
                    .set_num_files_found(&self.config_handler, 0)
                    .expect("Failed to change config");
                self.config
                    .set_files_scanned(&self.config_handler, 0)
                    .expect("Failed to change config");
                self.config
                    .set_tracks_found(&self.config_handler, 0)
                    .expect("Failed to change config");
                self.config
                    .set_albums_found(&self.config_handler, 0)
                    .expect("Failed to change config");

                // Albums: Full reset
                let album = self
                    .nav
                    .data_mut::<Page>(self.albumsid)
                    .expect("Should always be intialized");

                if let Page::Albums(page) = album {
                    page.albums = Arc::from(vec![]);
                    page.page_state = AlbumPageState::Loading
                }

                // Tracks: Full reset

                let tracks = self
                    .nav
                    .data_mut::<Page>(self.tracksid)
                    .expect("Should always be intialized");
                if let Page::Tracks(page) = tracks {
                    page.track_page_state = TrackPageState::Loading
                }

                // Playlists: Full reset
                if let Page::Playlists(page) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("Should always be intialized")
                {
                    page.playlist_page_state = PlaylistPageState::Loading
                }

                // Artists Reset
                if let Page::Artist(page) = self
                    .nav
                    .data_mut::<Page>(self.artistsid)
                    .expect("Should always be intialized")
                {
                    page.page_state = ArtistPageState::Loading
                }

                create_database();

                let path = self.config.scan_dir.clone().parse().unwrap();
                return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                    100,
                    |mut tx| async move {
                        scan_directory(path, &mut tx).await;
                        tx.send(Message::OnNavEnter(ReEnterNavReason::Rescan))
                            .await
                            .expect("de")
                    },
                ))
                .map(cosmic::Action::App);
            }
            Message::AddToDatabase(path) => {
                return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                    100,
                    move |mut tx| async move {
                        let file = fs::File::open(&path).unwrap();
                        let probe = get_probe();
                        let mss = symphonia::core::io::MediaSourceStream::new(
                            Box::new(file),
                            Default::default(),
                        );

                        if let Ok(mut reader) = probe.format(
                            &Default::default(),
                            mss,
                            &Default::default(),
                            &Default::default(),
                        ) {
                            if let Some(mdat) = reader.metadata.get() {
                                let tags = mdat
                                    .current()
                                    .unwrap()
                                    .tags()
                                    .iter()
                                    .filter(|a| a.is_known())
                                    .map(|a| a.clone())
                                    .collect();
                                create_database_entry(tags, &path).await;
                                tx.send(Message::UpdateScanProgress).await.unwrap();
                            } else {
                                let mdat = reader.format.metadata();
                                let tags = mdat
                                    .current()
                                    .unwrap()
                                    .tags()
                                    .iter()
                                    .filter(|a| a.is_known())
                                    .map(|a| a.clone())
                                    .collect();
                                create_database_entry(tags, &path).await;
                                tx.send(Message::UpdateScanProgress).await.unwrap();
                            }
                        } else {
                            if path.with_extension("m3u") == path
                                || path.with_extension("m3u8") == path
                            {
                                let mut dir = PathBuf::new();

                                if dirs::data_local_dir()
                                    .unwrap()
                                    .join(crate::app::AppModel::APP_ID)
                                    .join("Playlists")
                                    .exists()
                                {
                                    dir = dirs::data_local_dir()
                                        .unwrap()
                                        .join(crate::app::AppModel::APP_ID)
                                        .join("Playlists");
                                } else {
                                    match std::fs::create_dir(
                                        dirs::data_local_dir()
                                            .unwrap()
                                            .join(crate::app::AppModel::APP_ID)
                                            .join("Playlists"),
                                    ) {
                                        Ok(_) => {
                                            dir = dirs::data_local_dir()
                                                .unwrap()
                                                .join(crate::app::AppModel::APP_ID)
                                                .join("Playlists");
                                        }
                                        Err(err) => {
                                            tx.send(Message::ToastError(err.to_string()))
                                                .await
                                                .unwrap();
                                        }
                                    }
                                }

                                let name = path.file_name().unwrap().to_string_lossy().to_string();
                                fs::copy(path, dir.as_path().join(name)).unwrap();
                                tx.send(Message::UpdateScanProgress).await.unwrap();
                            } else {
                                tx.send(Message::ProbeFail).await.unwrap();

                                log::info!(
                                    "ERROR: Probe failure \nErred Path: {}",
                                    path.to_str().unwrap().to_string()
                                );
                            }
                        }
                    },
                ))
                .map(cosmic::Action::App);
            }
            Message::UpdateScanProgress => {
                self.config
                    .set_files_scanned(&self.config_handler, self.config.files_scanned + 1)
                    .expect("Failed to save to config");
            }
            Message::ProbeFail => {
                self.config
                    .set_num_files_found(&self.config_handler, self.config.num_files_found - 1)
                    .expect("Failed to save to config");
            }
            Message::UpdateScanDirSize => {
                self.config
                    .set_num_files_found(&self.config_handler, self.config.num_files_found + 1)
                    .expect("Config Save Failed");
            }

            // PAGE TASK RESPONSES
            Message::AlbumProcessed(new_album) => {
                if let Page::Albums(dat) = self
                    .nav
                    .data_mut::<Page>(self.albumsid)
                    .expect("Should always be intialized")
                {
                    dat.albums = Arc::from(new_album)
                }
            }
            Message::OnNavEnter(reasoning) => {
                //always
                self.search_field = "".to_string();

                // re-entered nav because:
                match reasoning {
                    ReEnterNavReason::UserInteraction => {
                        // normal
                    }
                    ReEnterNavReason::Rescan => {
                        // rescan must have finished, make it available again
                        self.rescan_available = true;
                    }
                    ReEnterNavReason::ArtistEdit => {
                        // an artist was edited
                        // todo: less obtrusive reloading of the page
                    }
                    ReEnterNavReason::PlaylistEdit => {
                        // a playlist was edited
                        // todo: less obtrusive reloading of the page
                    }
                }

                match self.nav.active_data_mut().unwrap() {
                    Page::NowPlaying(_) => {}
                    Page::Albums(val) => {}
                    Page::Playlists(page) => {}
                    Page::Tracks(page) => {
                        let mut page_ref = page.clone();
                        return cosmic::Task::future(async move {
                            let conn = connect_to_db();

                            let mut stmt = conn.prepare(
                                "
                                select track.id as id, track.name as title, art.name as artist, track.path, a.name as album_title
                                from track
                                    left join main.album_tracks at on track.id = at.track_id
                                    left join main.artists art on track.artist_id = art.id
                                    left join main.album a on at.album_id = a.id;
                            ").unwrap();

                            let tracks = stmt.query_map([], |row| {
                                Ok(
                                    AppTrack {
                                        id: row.get("id").unwrap_or(0),
                                        title: row
                                            .get("title")
                                            .unwrap_or("N/A".to_string()),
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
                                        cover_art: CoverArt::None,
                                    }
                                )
                            }).expect("Should never break");

                            let tracks = tracks.filter_map(|a| a.ok()).collect::<Vec<AppTrack>>();
                            log::info!("{:?}", tracks);

                            Message::PageDataRecieved(tracks)
                        }).map(cosmic::Action::App);
                    }
                    Page::Artist(page) => {}
                    Page::Genre(page) => {}
                }
            }
            Message::PageDataRecieved(tracks) => {
                let dats = self.nav.active_data_mut::<Page>().unwrap();
                if let Page::Tracks(dat) = dats {
                    dat.tracks = Arc::from(tracks);
                }

                self.update(Message::SearchClear);
            }

            Message::ArtistRequested(artist) => {
                let conn = rusqlite::Connection::open(
                    dirs::data_local_dir()
                        .unwrap()
                        .join(Self::APP_ID)
                        .join("nova_music.db"),
                )
                .unwrap();

                let mut stmt = conn.prepare(
                    "
                    SELECT a.id, a.name as name, a.album_cover as cover, art.name as artist, a.disc_number as dn, a.track_number as tn
                    FROM album a
                             left JOIN artists art ON a.artist_id = art.id
                    Where art.name = ?"
                ).expect("SQL is wrong");

                let val = stmt
                    .query_map([&artist], |row| {
                        Ok(Album {
                            name: row.get("name").expect("get name fail"),
                            artist: row.get("artist").expect("get artist fail"),
                            cover_art: match row.get::<_, Vec<u8>>("cover") {
                                Ok(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
                                Err(_) => None,
                            },
                            disc_number: row.get("dn").expect("get disc number fail"),
                            track_number: row.get("tn").expect("get track number fail"),
                        })
                    })
                    .unwrap();

                //todo single really only needs a reference to track_id
                let mut stmt = conn
                    .prepare(
                        "
SELECT t.id as track_id, t.name as name, a.name as artist, s.cover as cover
FROM single s
    left join track t on s.track_id = t.id
    left join artists a on t.artist_id = a.id
where a.name = ?    ",
                    )
                    .expect("SQL is wrong");

                let albums: Vec<Album> = val
                    .into_iter()
                    .filter_map(|a| a.ok())
                    .collect::<Vec<Album>>();

                let val = stmt
                    .query_map([&artist], |row| {
                        Ok(DisplaySingle {
                            id: row.get("track_id").unwrap_or(0),
                            title: row.get("name").expect("get name fail"),
                            artist: row.get("artist").expect("get artist fail"),
                            cover_art: match row.get::<_, Vec<u8>>("cover") {
                                Ok(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
                                Err(_) => None,
                            },
                        })
                    })
                    .unwrap();

                let singles: Vec<DisplaySingle> = val
                    .into_iter()
                    .filter_map(|a| a.ok())
                    .collect::<Vec<DisplaySingle>>();

                let image = conn.query_row(
                    "select artistpfp from artists where name = ?",
                    [&artist],
                    |row| row.get::<_, Vec<u8>>("artistpfp"),
                );

                let new_page = ArtistPage {
                    artist: ArtistInfo {
                        name: artist,
                        path: String::from(""),
                        image: match image {
                            Ok(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
                            Err(err) => {
                                log::warn!("Possible error: {}", err);
                                None
                            }
                        },
                    },
                    singles,
                    albums,
                };

                if let Page::Artist(page) = self.nav.data_mut::<Page>(self.artistsid).unwrap() {
                    page.page_state = ArtistPageState::ArtistPage(new_page)
                }
            }

            Message::ArtistPageReturn => {
                if let Page::Artist(artistpage) = self
                    .nav
                    .data_mut::<Page>(self.artistsid)
                    .expect("should always be intialized")
                {
                    match artistpage.page_state {
                        ArtistPageState::Album(_) => {
                            artistpage.page_state = ArtistPageState::ArtistPage(
                                artistpage.artist_page_cache.clone().unwrap(),
                            );
                        }
                        _ => {
                            artistpage.page_state = ArtistPageState::Loaded;
                        }
                    }

                    artistpage.artist_page_cache = None;
                }
            }
            Message::ArtistsLoaded(artists) => {
                if let Page::Artist(page) = self
                    .nav
                    .data_mut::<Page>(self.artistsid)
                    .expect("Should always be initialized")
                {
                    page.artists = artists;
                    page.page_state = ArtistPageState::Loaded
                }
            }

            Message::PlaylistFound(playlists) => {
                if let Page::Playlists(page) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("Should always be intialized")
                {
                    page.playlists = Arc::new(playlists);
                    page.playlist_page_state = PlaylistPageState::Loaded;
                }
            }
            Message::PlaylistSelected(playlist) => {
                if let Page::Playlists(page) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("should always be intialized")
                {
                    let mut tracks = vec![];

                    let files = io::BufReader::new(match fs::File::open(&playlist.path) {
                        Ok(val) => val,
                        Err(err) => {
                            log::info!("{}", err);
                            return cosmic::task::none();
                        }
                    });
                    let mut is_m3u = false;
                    let mut track_title = None;

                    for (index, line) in files.lines().filter_map(Result::ok).enumerate() {
                        if !is_m3u {
                            if line.contains("#EXTM3U") && index == 0 {
                                is_m3u = true;
                                log::info!("is m3u")
                            } else {
                                continue;
                            }
                        }

                        if is_m3u && line.contains("#EXTINF:") {
                            let title_divide = line.find(" - ").unwrap();
                            let line = line[title_divide + 2..].to_string();
                            track_title = Some(line);
                            continue;
                        } else {
                        }

                        if is_m3u && track_title.is_some() {
                            let path = PathBuf::from(line);

                            tracks.push(PlaylistTrack {
                                title: track_title.take().unwrap().parse().unwrap(),
                                path: path.to_string_lossy().to_string(),
                            });
                        }
                    }

                    let lpay = FullPlaylist { playlist, tracks };
                    page.playlist_page_state = PlaylistPageState::PlaylistPage(lpay);
                }
            }
            Message::PlaylistPageReturn => {
                if let Page::Playlists(page) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("should always be intialized")
                {
                    page.playlist_page_state = PlaylistPageState::Loaded;
                }
            }
            Message::TrackLoaded(track) => {
                if let Page::Tracks(dat) = self
                    .nav
                    .data_mut::<Page>(self.tracksid)
                    .expect("Should always be intialized")
                {
                    dat.tracks = Arc::new(track)
                }
            }
            Message::TracksLoaded => {
                if let Page::Tracks(dat) = self
                    .nav
                    .data_mut::<Page>(self.tracksid)
                    .expect("Should always be intialized")
                {
                    dat.track_page_state = TrackPageState::Loaded;
                }
            }
            Message::AlbumsLoaded => {
                if let Page::Albums(dat) = self
                    .nav
                    .data_mut::<Page>(self.albumsid)
                    .expect("should always be init")
                {
                    dat.page_state = AlbumPageState::Loaded;
                    dat.has_fully_loaded = true;
                }
            }
            Message::AlbumPageReturn => {
                if let Page::Albums(dat) = self
                    .nav
                    .data_mut::<Page>(self.albumsid)
                    .expect("Should always be intialized")
                {
                    match dat.has_fully_loaded {
                        true => {
                            dat.page_state = AlbumPageState::Loaded;
                        }
                        false => {
                            dat.page_state = AlbumPageState::Loading;
                        }
                    }
                    if let Some(view) = dat.viewport {
                        return cosmic::iced_widget::scrollable::scroll_to(
                            dat.scrollbar_id.clone(),
                            view.absolute_offset(),
                        );
                    } else {
                        return cosmic::task::none();
                    }
                }
            }
            Message::AlbumPageStateAlbum(new_page) => {
                match self
                    .nav
                    .active_data_mut::<Page>()
                    .expect("Should always be intialized")
                {
                    Page::Albums(old_page) => {
                        *old_page = new_page;
                    }
                    _ => {}
                }
            }
            Message::AlbumInfoRetrieved(fullalbum) => {
                log::info!("Album info retrieved: {:?}", fullalbum,);

                match self.nav.active_data_mut::<Page>().unwrap() {
                    Page::Artist(page) => page.page_state = ArtistPageState::Album(fullalbum),
                    Page::Albums(page) => {
                        page.page_state = AlbumPageState::Album(fullalbum);
                    }
                    _ => log::error!("Accessing page from a strange state"),
                }
            }
            Message::AlbumRequested(dat) => {
                todo!()
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
            Message::SeekFinished => self.sink.set_volume(self.config.volume / 100.0),
            app::Message::AddTrackToQueue(filepath) => {
                let pos = self.nav.entity_at(0).expect("REASON");
                let home_page = self.nav.data_mut::<Page>(pos).unwrap();
                if let Page::NowPlaying(_) = home_page {
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

                    if let Ok(track) = stmt.query_row([&filepath], |row| {
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
                                Ok(val) => {
                                    SomeLoaded(cosmic::widget::image::Handle::from_bytes(val))
                                }
                                Err(_) => CoverArt::None,
                            },
                        })
                    }) {
                        self.queue.push(track);
                    } else {
                        return self
                            .toasts
                            .push(cosmic::widget::toaster::Toast::new(format!(
                                "Track at \"{}\" not found in database",
                                filepath
                            )))
                            .map(cosmic::Action::App);
                    }
                }

                if self.sink.empty() {
                    let file = match std::fs::File::open(&filepath) {
                        Ok(file) => file,
                        Err(err) => {
                            log::error!("Error: {}", err);

                            return self
                                .toasts
                                .push(cosmic::widget::toaster::Toast::new(format!(
                                    "Track found in database but not at the filepath: {}",
                                    filepath.to_string()
                                )))
                                .map(cosmic::Action::App);
                        }
                    };

                    let decoder = rodio::Decoder::builder()
                        .with_byte_len(file.metadata().unwrap().len())
                        .with_data(file)
                        .with_gapless(true)
                        .with_seekable(true)
                        .build()
                        .expect("Failed to build decoder");

                    self.song_duration = decoder.total_duration().map(|val| val.as_secs_f64());
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
                        cosmic::task::batch(vec![progress_thread.0, sleeping_thread.0]).abortable();
                    match &mut self.task_handle {
                        None => self.task_handle = Some(vec![handle]),
                        Some(handles) => handles.push(handle),
                    }
                    self.sink.play();
                    return task;
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
                    .with_gapless(true)
                    .with_seekable(true)
                    .build()
                    .expect("Failed to build decoder");

                self.song_duration = Some(decoder.total_duration().unwrap().as_secs_f64());
                self.sink.append(decoder);

                let task_sink = Arc::clone(&self.sink);

                return cosmic::task::future(async move {
                    Message::SongFinished(
                        tokio::task::spawn_blocking(move || {
                            task_sink.sleep_until_end();
                            QueueUpdateReason::None
                        })
                        .await
                        .expect("nova_music.db"),
                    )
                });
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
            app::Message::AddAlbumToQueue(mut paths) => {
                return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                    0,
                    |mut tx| async move {
                        paths.sort_by(|a, b| a.1.cmp(&b.1));

                        for file in paths {
                            tx.send(Message::AddTrackToQueue(file.0))
                                .await
                                .expect("send")
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
            Message::AddToPlaylist => self.playlist_creation_dialog = true,
            Message::EditPlaylistCancel => self.playlist_edit_dialog = false,
            Message::EditPlaylistConfirm => {
                if PathBuf::from(&self.playlist_dialog_path).exists() {
                    let mut file = File::open(&self.playlist_dialog_path).unwrap();
                    let mut new_file = String::new();

                    let mut append_cover_path = true;
                    let mut append_name = true;

                    let mut string: String = String::from("");
                    file.read_to_string(&mut string).unwrap();

                    for entry in string.lines() {
                        let mut new_line = entry.to_string();
                        match entry.find("#PLAYLIST:") {
                            None => {
                                log::info!("no playlist name");
                            }
                            Some(_) => {
                                append_name = false;
                                log::info!("found and adding");
                                entry.clear();
                                new_line =
                                    format!("#PLAYLIST:{}", self.playlist_dialog_text.as_str());
                            }
                        }

                        match entry.find("#EXTALBUMARTURL:") {
                            None => {
                                if self.playlist_cover.is_some() {
                                    append_cover_path = true;
                                } else {
                                    append_cover_path = false;
                                }
                            }
                            Some(_) => {
                                append_cover_path = false;
                                log::info!("found and adding");
                                entry.clear();

                                if let Some(path) = self.playlist_cover.take() {
                                    new_line = String::from(format!(
                                        "#EXTALBUMARTURL:{}",
                                        path.to_str().unwrap()
                                    ))
                                } else {
                                    new_line = String::from("blank")
                                }
                            }
                        }

                        if new_line.contains("blank") {
                            new_line = String::from("")
                        } else {
                            new_line = format!("{}\n", new_line);
                        }

                        new_file.push_str(&new_line);
                    }
                    let mut file = File::create(&self.playlist_dialog_path).unwrap();
                    if append_cover_path {
                        new_file.push_str(
                            format!(
                                "#EXTALBUMARTURL:{}\n",
                                self.playlist_cover.take().unwrap().to_str().unwrap()
                            )
                            .as_str(),
                        )
                    }

                    if append_name {
                        new_file
                            .push_str(format!("#PLAYLIST:{}\n", self.playlist_dialog_text).as_str())
                    }
                    file.write_all(new_file.as_bytes()).unwrap()
                } else {
                    return self
                        .toasts
                        .push(cosmic::widget::toaster::Toast::new(
                            "Playlist path no longer exists",
                        ))
                        .map(cosmic::Action::App);
                }
                self.playlist_edit_dialog = false;

                if let Page::Playlists(val) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("should always be intitalized")
                {
                    val.playlist_page_state = PlaylistPageState::Loading
                }
                return cosmic::task::future(async move {
                    Message::OnNavEnter(ReEnterNavReason::PlaylistEdit)
                });
            }
            Message::CreatePlaylistIconChosen(path) => {
                log::info!("Image: {}", path.to_string_lossy());
                self.playlist_cover = Some(path)
            }
            Message::CreatePlaylistAddThumbnail => {
                return cosmic::task::future(async move {
                    let dialog = cosmic::dialog::file_chooser::open::Dialog::new();
                    match dialog.open_file().await {
                        Ok(selected) => {
                            let fp = selected.url().to_owned().to_file_path();

                            if let Ok(file) = fp {
                                Message::CreatePlaylistIconChosen(file)
                            } else {
                                Message::FolderPickerFail(String::from("Not a valid filepath"))
                            }
                        }
                        Err(err) => {
                            // todo toasts for file picking errors
                            let error: String;
                            match err {
                                Error::Cancelled => {
                                    error = String::from(
                                        "Cancelled File Picker: Keeping scan directory the same",
                                    )
                                }
                                Error::Close(err) => {
                                    error = String::from(format!("Closer: {}", err.to_string()))
                                }
                                Error::Open(err) => {
                                    error = String::from(format!("Open: {}", err.to_string()))
                                }
                                Error::Response(err) => {
                                    error = String::from(format!("Response: {}", err.to_string()))
                                }
                                Error::Save(err) => {
                                    error = String::from(format!("Save: {}", err.to_string()))
                                }
                                Error::SetDirectory(err) => {
                                    error =
                                        String::from(format!("Set Directory: {}", err.to_string()))
                                }
                                Error::SetAbsolutePath(err) => {
                                    error = String::from(format!(
                                        "Set Absolute Path: {}",
                                        err.to_string()
                                    ))
                                }
                                Error::UrlAbsolute => error = String::from("URL Absolute"),
                            }
                            Message::FolderPickerFail(error)
                        }
                    }
                })
                .map(action::Action::App);
            }

            Message::CreatePlaylistCancel => {
                self.playlist_creation_dialog = false;
            }

            Message::CreatePlaylistConfirm => {
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
                    .write_all(
                        format!(
                            "#EXTM3U \n#PLAYLIST:{}\n#EXTALBUMARTURL:{}\n",
                            self.playlist_dialog_text,
                            self.playlist_cover
                                .take()
                                .unwrap_or("".parse().unwrap())
                                .to_string_lossy()
                                .to_string()
                        )
                        .as_bytes(),
                    )
                    .expect("Failed to write Playlist file");
                for track in &self.queue {
                    new_file
                        .write_all(
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

                self.playlist_dialog_text = String::from("");
                self.playlist_cover = None;
                self.playlist_creation_dialog = false;
                if let Page::Playlists(page) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("should always be intialized")
                {
                    page.playlist_page_state = PlaylistPageState::Loading
                }
            }
            Message::SearchResults => {
                todo!()
            }
            Message::ToggleTitle(val) => {
                if let Page::Tracks(page) = self
                    .nav
                    .data_mut::<Page>(self.tracksid)
                    .expect("Should always be initialized")
                {
                    page.search_by_title = val
                }
            }
            Message::ToggleAlbum(val) => {
                if let Page::Tracks(page) = self
                    .nav
                    .data_mut::<Page>(self.tracksid)
                    .expect("Should always be initialized")
                {
                    page.search_by_album = val
                }
            }
            Message::ToggleArtist(val) => {
                if let Page::Tracks(page) = self
                    .nav
                    .data_mut::<Page>(self.tracksid)
                    .expect("Should always be intialized")
                {
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
            Message::ToggleFooter(val) => {
                self.config
                    .set_footer(&self.config_handler, val)
                    .expect("Failed to edit config");
            }
            app::Message::AddTrackById((t_type, id)) => {
                let conn = connect_to_db();

                let mut stmt =
                    "
                                select track.id as id, track.name as title, art.name as artist, track.path as path, a.album_cover, a.name as album_title
                                from track
                                left join main.album_tracks at on track.id = at.track_id
                                left join main.artists art on track.artist_id = art.id
                                left join main.album a on at.album_id = a.id
                                where track.id = ?
                            ";

                if let Ok(result) = conn.query_row(stmt, [&id], |row| {
                    let filepath = PathBuf::from(row.get::<_, String>("path").unwrap());
                    let visual = find_visual(&filepath);

                    Ok(AppTrack {
                        id: row.get("id").unwrap(),
                        artist: row.get("artist").unwrap(),
                        path_buf: filepath,
                        title: row.get("title").unwrap(),
                        album_title: match t_type {
                            TrackType::AlbumTrack => {
                                row.get("album_title").unwrap_or(String::from(""))
                            }
                            TrackType::Single => String::from(""),
                        },
                        cover_art: match visual {
                            Some(cover) => {
                                SomeLoaded(cosmic::widget::image::Handle::from_bytes(cover))
                            }
                            None => CoverArt::None,
                        },
                    })
                }) {
                    self.queue.push(result)
                }

                if self.sink.empty() {
                    let file = std::fs::File::open(self.queue.get(0).unwrap().path_buf.clone())
                        .expect("Failed to open file");

                    let decoder = rodio::Decoder::builder()
                        .with_byte_len(file.metadata().unwrap().len())
                        .with_data(file)
                        .with_gapless(true)
                        .with_seekable(true)
                        .build()
                        .expect("Failed to build decoder");

                    self.song_duration = decoder.total_duration().map(|val| val.as_secs_f64());
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
                        cosmic::task::batch(vec![progress_thread.0, sleeping_thread.0]).abortable();
                    match &mut self.task_handle {
                        None => self.task_handle = Some(vec![handle]),
                        Some(handles) => handles.push(handle),
                    }
                    self.sink.play();
                    return task;
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

    fn subscription(&self) -> cosmic::iced::Subscription<Self::Message> {
        struct MPRISSubscription;

        let mpris = cosmic::iced::Subscription::run_with_id(
            TypeId::of::<MPRISSubscription>(),
            cosmic::iced_futures::stream::channel(1, |mut output| async move {
                let rootinterface = MPRISRootInterface::new();
                let rootlistener = rootinterface.event.listen();

                let playerinterface = MPRISPlayer::new();
                let playerlistener = playerinterface.event.clone();

                let connection = connection::Builder::session()
                    .unwrap()
                    .name("org.mpris.MediaPlayer2.NovaMusic")
                    .unwrap()
                    .serve_at("/org/mpris/MediaPlayer2", rootinterface)
                    .unwrap()
                    .serve_at("/org/mpris/MediaPlayer2", playerinterface)
                    .unwrap()
                    .build()
                    .await
                    .unwrap();

                loop {
                    let listener = playerlistener.listen();
                    listener.await;
                    log::info!("Something was requested of player")
                }
            }),
        );
        cosmic::iced::Subscription::batch(vec![
            // Watch for application configuration changes.
            cosmic::iced::event::listen_with(handle_keybinds),
            mpris,
        ])
    }
}

fn handle_keybinds(
    event: cosmic::iced::event::Event,
    a: cosmic::iced::event::Status,
    _: cosmic::iced::window::Id,
) -> Option<Message> {
    if let cosmic::iced::event::Status::Captured = a {
        return None;
    }

    match event {
        Event::Keyboard(key) => {
            if let keyboard::Event::KeyPressed { key, .. } = key {
                log::info!("[{:?}]", key);
                match key {
                    cosmic::iced::keyboard::Key::Named(
                        cosmic::iced::keyboard::key::Named::Space,
                    ) => return Some(Message::PlayPause),

                    cosmic::iced::keyboard::Key::Named(
                        cosmic::iced::keyboard::key::Named::MediaSkipBackward,
                    ) => return Some(Message::PreviousTrack),

                    cosmic::iced::keyboard::Key::Named(
                        cosmic::iced::keyboard::key::Named::MediaSkipForward,
                    ) => return Some(Message::SkipTrack),

                    cosmic::iced::keyboard::Key::Named(
                        cosmic::iced::keyboard::key::Named::MediaPlayPause,
                    ) => return Some(Message::PlayPause),

                    _ => None,
                }
            } else {
                None
            }
        }
        _ => None,
    }
}

impl AppModel {
    /// The about page for this app.
    pub fn about<'a>(&self) -> Element<'a, Message> {
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
            return cosmic::task::future(async move {
                Message::OnNavEnter(ReEnterNavReason::UserInteraction)
            });
        }

        if let Some(id) = self.core.main_window_id() {
            return cosmic::Task::batch(vec![self.set_window_title(window_title, id), do_thing()]);
        } else {
            Task::none()
        }
    }

    fn update_pageinfo(&mut self) {
        match self.nav.active_data::<Page>().unwrap() {
            Page::Tracks(page) => {
                if let TrackPageState::Search = page.track_page_state {
                    log::error!("NOT YET")
                } else {
                    log::info!("PROPER")
                }
            }
            _ => {
                log::error!("NOT YET")
            }
        }
    }
}

/// The page to display in the application.
#[derive(Debug)]
pub enum Page {
    NowPlaying(HomePage),
    Artist(ArtistsPage),
    Albums(AlbumPage),
    Playlists(PlaylistPage),
    Tracks(TrackPage),
    Genre(GenrePage),
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
    SortChange(SortBy),
}

impl menu::action::MenuAction for Action {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Action::About => Message::ToggleContextPage(ContextPage::About),
            Action::Settings => Message::ToggleContextPage(ContextPage::Settings),
            Action::SortChange(val) => Message::Sort(*val),
        }
    }
}

fn connect_to_db() -> rusqlite::Connection {
    let conn = match rusqlite::Connection::open(
        dirs::data_local_dir()
            .unwrap()
            .join("dev.lunarsrl.NovaMusic")
            .join("nova_music.db"),
    ) {
        Ok(conn) => conn,
        Err(err) => {
            panic!("{}", err)
        }
    };
    conn
}
