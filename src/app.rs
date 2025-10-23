// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::artists::ArtistInfo;
use cosmic::dialog::file_chooser::Error;
use regex::Regex;

use rayon::iter::IndexedParallelIterator;

use rayon::iter::{IntoParallelRefIterator, ParallelIterator};
mod albums;
mod artists;
pub(crate) mod home;
mod playlists;
mod scan;
mod settings;
mod tracks;

use crate::app::albums::{
    get_album_info, get_top_album_info, Album, AlbumPage, AlbumPageState, FullAlbum,
};

use crate::app::artists::ArtistPageState::ArtistPage;
use crate::app::artists::{ArtistPageState, ArtistsPage};
use crate::app::home::HomePage;
use crate::app::playlists::{
    FullPlaylist, Playlist, PlaylistPage, PlaylistPageState, PlaylistTrack,
};
use crate::app::scan::scan_directory;
use crate::app::tracks::{SearchResult, TrackPage, TrackPageState};
use crate::config::Config;
use crate::database::{create_database, create_database_entry};
use crate::{app, config, fl};
use colored::Colorize;
use cosmic::app::context_drawer;
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::task::Handle;
use cosmic::iced::window::Id;
use cosmic::iced::Alignment::Start;
use cosmic::iced::{Alignment, Color, ContentFit, Length};
use cosmic::iced_widget::scrollable::Viewport;
use cosmic::prelude::*;
use cosmic::widget::{self, icon, menu, nav_bar};
use cosmic::{action, cosmic_config, cosmic_theme, theme};
use futures_util::{SinkExt, StreamExt};
use rodio::{Sink, Source};
use rusqlite::fallible_iterator::FallibleIterator;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs::File;
use std::io::{BufRead, Read, Write as OtherWrite};
use std::path::PathBuf;
use std::sync::Arc;
use std::thread::sleep;
use std::time::Duration;
use std::{fs, io};
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
    pub search_field: String,
    pub playlist_dialog_text: String,
    playlist_dialog_path: String,
    pub playlist_cover: Option<PathBuf>,
    footer_toggled: bool,
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
    pub cover_art: Option<cosmic::widget::image::Handle>,
}

/// Minimum amount of info required to display fully expose a Single track
#[derive(Debug)]
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
pub enum Message {
    OpenRepositoryUrl,
    ToggleContextPage(ContextPage),
    _UpdateConfig(Config),
    LaunchUrl(String),

    // Config change related
    RescanDir,

    // Filesystem scan related
    ChooseFolder,
    FolderChosen(String),
    FolderPickerFail(String),
    UpdateScanProgress,
    UpdateScanDirSize,
    AddToDatabase(PathBuf),
    ProbeFail,

    // Page Rendering
    OnNavEnter,
    ScrollView(Viewport),

    // Album Page
    AlbumRequested((String, String)), // when an album icon is clicked [gets title & artist of album]
    AlbumInfoRetrieved(FullAlbum), // when task assigned to retrieving requested albums info is completed [gets full track list of album]
    AlbumProcessed(Vec<Album>), // when an album retrieved from db's data is organized and ready [Supplies AlbumPage with the new Album]
    AlbumsLoaded, // when albums table retrieved from db is exhausted after OnNavEnter in Album Page [Sets page state to loaded]
    AlbumPageStateAlbum(AlbumPage), // when album info is retrieved [Replaces AlbumPage with AlbumPage with new info]
    AlbumPageReturn,

    // Home Page
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
    FooterToggle,

    // Error Reporting
    Toasts(cosmic::widget::toaster::ToastId),
    ToastError(String),

    //experimenting
    CreatePlaylistCancel,
    CreatePlaylistAddThumbnail,
    CreatePlaylistIconChosen(PathBuf),
    PlaylistEdit(String),
    EditPlaylistConfirm,
    EditPlaylistCancel,
}

#[derive(Clone, Debug)]
pub enum QueueUpdateReason {
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
    const APP_ID: &'static str = "dev.riveroluna.NovaMusic";

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
            .data::<Page>(Page::Albums(AlbumPage::new(vec![])))
            .icon(icon::from_name("media-optical-symbolic"))
            .id();

        let playlistsid = nav
            .insert()
            .text(fl!("playlists"))
            .data::<Page>(Page::Playlists(PlaylistPage::new()))
            .icon(icon::from_name("playlist-symbolic"))
            .id();

        // INIT CONFIG
        let config = config::Config::load();
        let config_handler = match config.0 {
            None => {
                panic!("NO CONFIG");
            }
            Some(som) => som,
        };
        let config = config.1;

        // init toasts

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
            search_field: "".to_string(),

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
            footer_toggled: true,

            toasts: cosmic::widget::toaster::Toasts::new(|a| Message::Toasts(a)),
            albumsid,
            tracksid,
            artistsid,
            playlistsid,
            homeid,
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
                                cosmic::widget::text::heading(data.0.unwrap_or("")).into(),
                                cosmic::widget::text::heading(data.1.unwrap_or("")).into(),
                                cosmic::widget::text::heading(data.2.unwrap_or("")).into(),
                                cosmic::widget::horizontal_space().into(),
                                // todo Find a good way of letting users clear the queue from the footer
                                // cosmic::widget::button::destructive(fl!("ClearAll"))
                                //     .on_press(Message::ClearQueue)
                                //     .into(),
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
                    .title(fl!("firsttimetitle"))
                    .body(fl!("firsttimebody"))
                    .control(cosmic::widget::container(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(format!(
                                "{}: {}",
                                fl!("currentdir"),
                                self.config.scan_dir.as_str()
                            ))
                            .into(),
                            cosmic::widget::horizontal_space().into(),
                            cosmic::widget::button::text(fl!("folderselect"))
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

        // Dialogs from page user interactions
        match self.nav.active_data::<Page>().unwrap() {
            Page::NowPlaying(_) => {}
            Page::Artist(page) => {
                if self.artistpage_edit_dialog {
                    return Some(page.artist_edit_dialog().into());
                }
            }
            Page::Albums(_) => {}
            Page::Playlists(val) => {
                let icon = match &self.playlist_cover {
                    None => cosmic::widget::container(
                        cosmic::widget::button::icon(
                            cosmic::widget::icon::from_name("view-list-images-symbolic")
                                .size(6 * 8),
                        )
                        .padding(cosmic::theme::spacing().space_s)
                        .on_press(Message::CreatePlaylistAddThumbnail)
                        .class(cosmic::theme::Button::Suggested),
                    )
                    .class(cosmic::theme::Container::Secondary)
                    .width(Length::Fixed(6.0 * 16.0))
                    .height(Length::Fixed(6.0 * 16.0))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into(),
                    Some(val) => cosmic::widget::container(
                        cosmic::widget::button::custom_image_button(
                            cosmic::widget::image(cosmic::widget::image::Handle::from_path(val))
                                .content_fit(ContentFit::Fill),
                            None,
                        )
                        .on_press(Message::CreatePlaylistAddThumbnail),
                    )
                    .width(Length::Fixed(6.0 * 16.0))
                    .height(Length::Fixed(6.0 * 16.0))
                    .align_x(Horizontal::Center)
                    .align_y(Vertical::Center)
                    .into(),
                };

                if self.playlist_creation_dialog {
                    return Some(
                        cosmic::widget::dialog::Dialog::new()
                            .title(fl!("DialogPlaylistTitle"))
                            .control(
                                cosmic::widget::container(
                                    cosmic::widget::row::with_children(vec![
                                        icon,
                                        cosmic::widget::text_input(
                                            fl!("PlaylistInputPlaceholder"),
                                            self.playlist_dialog_text.as_str(),
                                        )
                                        .on_input(|input| Message::UpdatePlaylistName(input))
                                        .into(),
                                    ])
                                    .align_y(Vertical::Bottom)
                                    .spacing(cosmic::theme::spacing().space_m),
                                )
                                .align_x(Horizontal::Center),
                            )
                            .primary_action(
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "object-select-symbolic",
                                ))
                                .class(cosmic::theme::Button::Suggested)
                                .on_press(Message::CreatePlaylistConfirm),
                            )
                            .secondary_action(
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "window-close-symbolic",
                                ))
                                .class(cosmic::theme::Button::Standard)
                                .on_press(Message::CreatePlaylistCancel),
                            )
                            .into(),
                    );
                }

                if self.playlist_edit_dialog {
                    return Some(
                        cosmic::widget::dialog::Dialog::new()
                            .title(fl!("DialogPlaylistEdit"))
                            .control(
                                cosmic::widget::container(
                                    cosmic::widget::row::with_children(vec![
                                        icon,
                                        cosmic::widget::text_input(
                                            fl!("PlaylistInputPlaceholder"),
                                            self.playlist_dialog_text.as_str(),
                                        )
                                        .on_input(|input| Message::UpdatePlaylistName(input))
                                        .into(),
                                    ])
                                    .align_y(Vertical::Bottom)
                                    .spacing(cosmic::theme::spacing().space_m),
                                )
                                .align_x(Horizontal::Center),
                            )
                            .primary_action(
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "object-select-symbolic",
                                ))
                                .class(cosmic::theme::Button::Suggested)
                                .on_press(Message::EditPlaylistConfirm),
                            )
                            .secondary_action(
                                cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                    "window-close-symbolic",
                                ))
                                .class(cosmic::theme::Button::Standard)
                                .on_press(Message::EditPlaylistCancel),
                            )
                            .into(),
                    );
                }
                // Dialog for deleting a playlist
                if self.playlist_delete_dialog {
                    if let PlaylistPageState::PlaylistPage(val) = &val.playlist_page_state {
                        return Some(
                            cosmic::widget::dialog::Dialog::new()
                                .title(fl!("DialogPlaylistDelete"))
                                .body(fl!(
                                    "DialogPlaylistDeleteClarify",
                                    path = val.playlist.path.as_str()
                                ))
                                .primary_action(
                                    cosmic::widget::button::text(fl!(
                                        "DialogPlaylistDeleteConfirm"
                                    ))
                                    .class(cosmic::theme::Button::Destructive)
                                    .on_press(Message::PlaylistDeleteConfirmed),
                                )
                                .secondary_action(
                                    cosmic::widget::button::text(fl!("Cancel"))
                                        .class(cosmic::theme::Button::Standard)
                                        .on_press(Message::PlaylistDeleteSafety),
                                )
                                .into(),
                        );
                    }
                }
            }
            Page::Tracks(_) => {}
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
            Page::Tracks(track_page) => body = track_page.load_page(self).explain(Color::WHITE),
            Page::Artist(artists_page) => body = artists_page.load_page(self),
            Page::Albums(album_page) => body = album_page.load_page(self).explain(Color::WHITE),
            Page::Playlists(playlist_page) => {
                body = playlist_page.load_page(self).explain(Color::WHITE)
            }
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
            },
            Message::PlaylistDeleteConfirmed => {
                if let Some(page) = self.nav.data_mut::<PlaylistPage>(self.playlistsid) {
                    if let PlaylistPageState::PlaylistPage(page) = &page.playlist_page_state {
                        match std::fs::remove_file(&page.playlist.path) {
                            Ok(_) => {
                                return cosmic::Task::future(async move { Message::OnNavEnter })
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
            Message::_UpdateConfig(config) => {
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
                        log::error!("Update Search Error: {:?}", err);
                        return self
                            .toasts
                            .push(cosmic::widget::toaster::Toast::new(fl!("SearchFailed")))
                            .map(cosmic::Action::App);
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
                    Page::Playlists(page) => {
                        let cloned_playlists = page.playlists.clone();

                        return cosmic::Task::stream(
                            cosmic::iced_futures::stream::channel(0, |mut tx| async move {
                                tokio::task::spawn_blocking(move || {
                                    let mut playlists = cloned_playlists
                                        .par_iter()
                                        .enumerate()
                                        .map(|(index, playlist)| {
                                            return match regex.find(&playlist.title) {
                                                None => SearchResult {
                                                    tracks_index: index,
                                                    score: 999,
                                                },
                                                Some(val) => {
                                                    if val.range().start == 0 {
                                                        if val.range().end == playlist.title.len() {
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

                                    playlists.sort_by(|a, b| a.score.cmp(&b.score));
                                    tx.try_send(Message::SearchResults(playlists))
                                });
                                ()
                            })
                            .map(action::Action::App),
                        );
                    }
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
                    Page::Artist(_) => {}
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

                // Playlists: FUll reset

                if let Page::Playlists(page) = self
                    .nav
                    .data_mut::<Page>(self.playlistsid)
                    .expect("Should always be intialized")
                {
                    page.playlist_page_state = PlaylistPageState::Loading
                }

                create_database();

                let path = self.config.scan_dir.clone().parse().unwrap();
                return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                    100,
                    |mut tx| async move {
                        scan_directory(path, &mut tx).await;
                        tx.send(Message::OnNavEnter).await.expect("de")
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
                self.rescan_available = true;
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
            Message::OnNavEnter => {
                self.search_field = "".to_string();

                match self.nav.active_data_mut().unwrap() {
                    Page::NowPlaying(_) => {}
                    Page::Albums(val) => {
                        if let AlbumPageState::Loading = val.page_state {
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
                                                            log::info!("Album iter error [on nav enter] : {}", e);
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
                    }
                    Page::Playlists(page) => {
                        match &page.playlist_page_state {
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

                                            for file in dir.flatten() {
                                                let files = io::BufReader::new(
                                                    fs::File::open(file.path()).unwrap(),
                                                );
                                                let path = file.path();
                                                let mut is_m3u = false;
                                                let mut title = String::from("");
                                                let mut cover_path = None;

                                                for (_, line) in
                                                    files.lines().map_while(Result::ok).enumerate()
                                                {
                                                    if line.contains("#EXTM3U") {
                                                        is_m3u = true;
                                                    }

                                                    if line.contains("#PLAYLIST:") && is_m3u {
                                                        title = line.replace("#PLAYLIST:", "");
                                                    } else if title.is_empty() {
                                                        title = path.file_name().unwrap().to_str().unwrap().to_string();
                                                    }

                                                    if line.contains("#EXTALBUMARTURL:") && cover_path.is_none() && is_m3u {
                                                        cover_path = Some(cosmic::widget::image::Handle::from_path(PathBuf::from(line.replace("#EXTALBUMARTURL:", ""))))
                                                    }
                                                }

                                                playlists.push(
                                                    Playlist {
                                                        title,
                                                        path: path.to_string_lossy().to_string(),
                                                        thumbnail: cover_path,
                                                    }
                                                )
                                            }
                                            tx.try_send(Message::PlaylistFound(playlists))
                                                .expect("send error");
                                        });
                                    },
                                ))
                                    .map(cosmic::Action::App);
                            }
                            PlaylistPageState::Loaded => {}
                            PlaylistPageState::PlaylistPage(_) => {}
                            PlaylistPageState::Search(_) => {
                                page.playlist_page_state = PlaylistPageState::Loaded
                            }
                        }
                    }
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
                                        let stmt = conn.prepare(
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
                    Page::Artist(page) => match &page.page_state {
                        ArtistPageState::Loading => {
                            return cosmic::Task::stream(
                                cosmic::iced_futures::stream::channel(100, |mut tx| async move {
                                    tokio::task::spawn_blocking(move || {
                                        let mut artists: Vec<ArtistInfo> = vec![];
                                        let conn = rusqlite::Connection::open(
                                            dirs::data_local_dir()
                                                .unwrap()
                                                .join(Self::APP_ID)
                                                .join("nova_music.db"),
                                        ).unwrap();

                                        let mut stmt = conn.prepare("select * from artists").expect("Statement Faulty @ OnNavEnter Artists");

                                        let rows = stmt.query_map([], |row| {
                                            let name = row.get::<_, String>("name").expect("Should be string");
                                            let regex = regex::RegexBuilder::new(r"feat\.|with|ft\.|&").case_insensitive(true).build().unwrap();


                                            match regex.is_match(&name) {
                                                false => {
                                                    return Ok(
                                                        ArtistInfo {
                                                            name: name,
                                                            image: match row.get::<_, Vec<u8>>("artistpfp") {
                                                                Ok(val) => {
                                                                    Some(cosmic::widget::image::Handle::from_bytes(val))
                                                                }
                                                                Err(e) => {
                                                                    log::warn!("Potential Error @ OnNavEnter Artists: {}", e);
                                                                    None
                                                                }
                                                            },
                                                        }
                                                    )
                                                }
                                                true => {
                                                    return Err(rusqlite::Error::UnwindingPanic)
                                                }
                                            };
                                        }).expect("Query map failed");

                                        artists = rows.into_iter().filter_map(|a| a.ok()).collect();


                                        tx.try_send(Message::ArtistsLoaded(artists))
                                    });
                                })
                                    .map(cosmic::Action::App),
                            )
                        }
                        ArtistPageState::Search(_) => {}
                        ArtistPageState::Loaded => {}
                        ArtistPageState::ArtistPage(page) => {}
                        ArtistPageState::ArtistPageSearch(_) => {}
                    },
                }
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

                let mut stmt = conn
                    .prepare(
                        "
                        SELECT s.id, s.name as name, s.cover as cover, art.name as artist
                        FROM single s
                        left JOIN artists art ON s.artist_id = art.id
                        Where art.name = ?
                    ",
                    )
                    .expect("SQL is wrong");

                let albums: Vec<Album> = val
                    .into_iter()
                    .filter_map(|a| a.ok())
                    .collect::<Vec<Album>>();

                let val = stmt
                    .query_map([&artist], |row| {
                        Ok(DisplaySingle {
                            id: row.get("id").unwrap_or(0),
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

                let new_page = crate::app::artists::ArtistPage {
                    artist: ArtistInfo {
                        name: artist,
                        image: None,
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
                    artistpage.page_state = ArtistPageState::Loaded;
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
            Message::AlbumInfoRetrieved(albuminfopage) => {
                log::info!("ALBUM INFO RETRIEVED: {:?}", albuminfopage);

                let album_page = self
                    .nav
                    .data_mut::<Page>(self.albumsid)
                    .expect("Should always be intialized");
                if let Page::Albums(page) = album_page {
                    page.page_state = AlbumPageState::Album(albuminfopage);
                }
            }
            Message::AlbumRequested(dat) => {
                match self
                    .nav
                    .active_data_mut::<Page>()
                    .expect("Should always be intialized")
                {
                    Page::Albums(_) => {
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
                                Ok(val) => Some(cosmic::widget::image::Handle::from_bytes(val)),
                                Err(_) => None,
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
                    let file = std::fs::File::open(filepath).expect("Failed to open file");

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
                return cosmic::task::future(async move { Message::OnNavEnter });
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
            Message::SearchResults(tracks) => {
                match self
                    .nav
                    .active_data_mut::<Page>()
                    .expect("Should always be intialized")
                {
                    Page::NowPlaying(_) => {}

                    Page::Albums(page) => page.page_state = AlbumPageState::Search(tracks),
                    Page::Playlists(page) => {
                        page.playlist_page_state = PlaylistPageState::Search(tracks)
                    }
                    Page::Tracks(track_list) => {
                        track_list.track_page_state = TrackPageState::Search;
                        track_list.search = tracks;
                    }
                    &mut Page::Artist(_) => todo!(),
                }
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
            Message::FooterToggle => {
                if let Page::Albums(page) = self
                    .nav
                    .data_mut::<Page>(self.albumsid)
                    .expect("Should always be intialized")
                {
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
            return cosmic::task::future(async move { Message::OnNavEnter });
        }

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
    Artist(ArtistsPage),
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
}

impl menu::action::MenuAction for Action {
    type Message = Message;

    fn message(&self) -> Self::Message {
        match self {
            Action::About => Message::ToggleContextPage(ContextPage::About),
            Action::Settings => Message::ToggleContextPage(ContextPage::Settings),
        }
    }
}
