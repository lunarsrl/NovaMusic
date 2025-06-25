// SPDX-License-Identifier: GPL-2.0-or-later

mod albums;
mod artists;
mod home;
mod playlists;
mod scan;
mod settings;

use tokio::task::spawn_blocking;

use crate::app::albums::{get_album_info, get_top_album_info, Album, AlbumPage, FullAlbum};
use crate::app::scan::{scan_directory, MediaFileTypes};
use crate::app::Message::{AlbumPageDone, SubscriptionChannel, UpdateScanProgress};
use crate::config::Config;
use crate::database::{create_database, create_database_entry};
use crate::{app, config, fl, StandardTagKeyExt};
use cosmic::app::context_drawer;
use cosmic::cosmic_config::{self, CosmicConfigEntry};
use cosmic::iced::alignment::{Horizontal, Vertical};
use cosmic::iced::keyboard::key::Physical::Code;
use cosmic::iced::wgpu::naga::FastHashMap;
use cosmic::iced::{alignment, stream, Alignment, ContentFit, Fill, Length, Pixels, Subscription};
use cosmic::prelude::*;
use cosmic::widget::segmented_button::Entity;
use cosmic::widget::{self, container, icon, menu, nav_bar, progress_bar, toaster, JustifyContent};
use cosmic::{cosmic_theme, iced, iced_futures, theme};
use futures_util::SinkExt;
use log::info;
use rodio::source::SeekError::SymphoniaDecoder;
use rodio::{OutputStream, OutputStreamHandle};
use rusqlite::fallible_iterator::FallibleIterator;
use std::collections::HashMap;
use std::fmt::Debug;
use std::fs;
use std::future::Future;
use std::path::{Path, PathBuf};

use cosmic::cosmic_theme::palette::cast::IntoComponents;
use cosmic::iced_core::text::Wrapping;
use std::thread::sleep;
use std::time::Duration;
use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_AAC, CODEC_TYPE_NULL};
use symphonia::core::formats::FormatOptions;
use symphonia::core::meta::{MetadataOptions, MetadataRevision, StandardTagKey, Tag, Value};
use symphonia::core::probe::Hint;
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

    // Pages
    pub ready_to_render: bool,

    //Settings Page
    pub change_dir_filed: String,
    pub rescan_available: bool,
    pub stream_handle: OutputStreamHandle,
    pub stream: OutputStream,
    //Album Page
    pub albums: Vec<Album>,
    //experimenting
}

/// Messages emitted by the application and its widgets.
#[derive(Debug, Clone)]
pub enum Message {
    OpenRepositoryUrl,
    SubscriptionChannel,
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
    PageSpecificTask,
    AlbumPageDone(AlbumPage),


    // Album Page
    AlbumRequested((String, String)),
    AlbumInfoRetrieved(FullAlbum),

    // Audio Messages
    StartStream(PathBuf),

    //experimenting
    ToastDone,
    OnNavEnter,
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

    /// Initializes the application with any given flags and startup commands.
    fn init(
        core: cosmic::Core,
        _flags: Self::Flags,
    ) -> (Self, Task<cosmic::Action<Self::Message>>) {
        // Create a nav bar with three page items.
        let mut nav = nav_bar::Model::default();
        let (stream, stream_handle) = OutputStream::try_default().unwrap();

        nav.insert()
            .text(fl!("home"))
            .data::<Page>(Page::NowPlaying)
            .icon(icon::from_name("applications-audio-symbolic"))
            .activate();

        nav.insert()
            .text(fl!("artists"))
            .data::<Page>(Page::Artists)
            .icon(icon::from_name("avatar-default-symbolic"));

        nav.insert()
            .text(fl!("albums"))
            .data::<Page>(Page::Albums(AlbumPage::new(None)))
            .icon(icon::from_name("media-optical-symbolic"));

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
            stream_handle,
            // Page Data
            ready_to_render: false,
            //experimenting
            albums: vec![],
        };

        // Create a startup command that sets the window title.
        let command = app.update_title();

        (app, command)
    }

    /// Elements to pack at the start of the header bar.
    fn header_start(&self) -> Vec<Element<Self::Message>> {
        let menu_bar = menu::bar(vec![menu::Tree::with_children(
            menu::root(fl!("view")),
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
            Page::NowPlaying => {
                cosmic::widget::container(cosmic::widget::text::title1("Now Playing")).into()
            }
            Page::Artists => {
                cosmic::widget::container(cosmic::widget::text::title1(" Artists ")).into()
            }
            Page::Albums(albumspage) => {
                albumspage.load_page()
            }
            Page::Playlists => {
                cosmic::widget::container(cosmic::widget::text::title1("Playlists")).into()
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

            Message::SubscriptionChannel => {
                log::info!("subscription channel");
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
                Ok(dir) => {
                    match self.config.set_scan_dir(&self.config_handler, val) {
                        Ok(val) => {
                        }
                        Err(err) => {
                           log::error!("dir: {:?}", err);
                        }
                    }
                }
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
                self.rescan_available = false;
                self.config.set_num_files_found(&self.config_handler, 0);
                self.config.set_files_scanned(&self.config_handler, 0);

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

                                    if let Some(mdat_revision) =
                                        reader.metadata.get().unwrap().current()
                                    {
                                        

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

            Message::StartStream(filepath) => {
                let file = std::fs::File::open(&filepath).unwrap();
                self.stream_handle
                    .play_once(file)
                    .expect("TODO: panic message");
            }
            Message::PageSpecificTask => {}

            // PAGE TASK RESPONSES
            Message::AlbumPageDone(newPage) => {
               match self.nav.active_data_mut::<Page>().unwrap() {
                   Page::NowPlaying => {}
                   Page::Artists => {}
                   Page::Albums(oldPage) => {
                       *oldPage = newPage
                   }
                   Page::Playlists => {}
               }
            }

            Message::OnNavEnter => match self.nav.active_data().unwrap() {
                Page::NowPlaying => {}
                Page::Artists => {}
                Page::Albums(page) => {
                    return cosmic::task::future(async move {
                        let albums = get_top_album_info().await;
                        AlbumPageDone(AlbumPage::new(Some(albums)))
                    });
                }
                Page::Playlists => {}
            },
            Message::AlbumInfoRetrieved(albuminfopage) => {
                return cosmic::task::future(async move {
                    
                    AlbumPageDone(AlbumPage::new_album_page(albuminfopage))
                });
            },
            Message::ToastDone => {}
            Message::AlbumRequested(dat) => {
                match self.nav.active_data_mut::<Page>().unwrap() {
                    Page::Albums(page_dat) => {
                        return cosmic::Task::stream(cosmic::iced_futures::stream::channel(
                            0,
                            |mut tx| async move {
                                let album = get_album_info(dat.0, dat.1).await;
                                tx.send(Message::AlbumInfoRetrieved(album)).await.expect("send")
                            }
                        )).map(cosmic::Action::App)
                    }
                    _ => {
                        // should never happen
                        log::error!("Requested album info while outside albums page somehow")
                    }
                }
            }
        }
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
        };

        if let Some(id) = self.core.main_window_id() {
            return cosmic::Task::batch(vec![self.set_window_title(window_title, id), do_thing()]);
        } else {
            Task::none()
        }
    }
}

/// The page to display in the application.
#[derive(Clone, Debug)]
pub enum Page {
    NowPlaying,
    Artists,
    Albums(AlbumPage),
    Playlists,
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
