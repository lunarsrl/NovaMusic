use crate::app::page::CoverArt;
use crate::app::page::CoverArt::SomeLoaded;
use crate::app::{connect_to_db, Message};
use crate::database::find_visual;
use cosmic::iced::alignment::Vertical;
use cosmic::iced::core::text::{Ellipsize, EllipsizeHeightLimit};
use cosmic::iced::{widget, Length};
use cosmic::Element;
use std::path::PathBuf;
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct QueuedTrack {
    pub id: u32,
    pub title: Arc<String>,
    pub path_buf: Arc<PathBuf>,
}

impl QueuedTrack {
    pub fn get_by_id(id: u32) -> Result<QueuedTrack, String> {
        let conn = connect_to_db();

        let mut stmt = "
                                select track.id as id, track.name as title, track.path as path
                                from track
                                where track.id = ?
                            ";

        match conn.query_row(stmt, [&id], |row| {
            let filepath = PathBuf::from(row.get::<_, String>("path").unwrap());
            Ok(QueuedTrack {
                id: row.get("id").unwrap(),
                path_buf: filepath.into(),
                title: row.get::<&str, String>("title").unwrap().into(),
            })
        }) {
            Ok(a) => Ok(a),
            Err(a) => Err(format!("Failed:{}", a).to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// All info associated with a track
pub struct AppTrack {
    pub id: u32,
    pub title: Arc<String>,
    pub artist: String,
    pub album_title: String,
    pub path_buf: Arc<PathBuf>,
    pub cover_art: CoverArt,
}

impl AppTrack {
    pub(crate) fn get_by_id(id: u32) -> Result<AppTrack, String> {
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

        match conn.query_row(stmt, [&id], |row| {
            let filepath = PathBuf::from(row.get::<_, String>("path").unwrap());
            let visual = find_visual(&filepath);

            Ok(AppTrack {
                id: row.get("id").unwrap(),
                artist: row.get("artist").unwrap(),
                path_buf: filepath.into(),
                title: row.get::<&str, String>("title").unwrap().into(),
                album_title: row.get("album_title").unwrap_or(String::from("")),
                cover_art: match visual {
                    Some(cover) => SomeLoaded(cosmic::widget::image::Handle::from_bytes(cover)),
                    None => CoverArt::None,
                },
            })
        }) {
            Ok(a) => Ok(a),
            Err(a) => Err(format!("Failed:{}", a).to_string()),
        }
    }

    pub fn display<'a>(self) -> Element<'a, Message> {
        let space_main = 300.0;
        let space_mod = 150.0;
        cosmic::widget::column::with_children(vec![cosmic::iced::widget::hover(
            // Normal Display
            cosmic::widget::container(cosmic::widget::column![
                cosmic::widget::divider::horizontal::default(),
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::column![
                        cosmic::widget::text::heading(self.title.to_string())
                            .width(Length::Fixed(space_main))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                        cosmic::widget::text::text(self.artist.to_string())
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                            .width(Length::Fixed(space_main)),
                        cosmic::widget::text::text(self.album_title.to_string())
                            .width(Length::Fixed(space_main))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                    ]
                    .into(),
                    // todo: custom extra info sections, with sorting capabilities
                    // cosmic::widget::button::text("Mod Entry 1")
                    //     .width(Length::Fixed(space_mod))
                    //     .into(),
                    // cosmic::widget::button::text("Mod Entry 2")
                    //     .width(Length::Fixed(space_mod))
                    //     .into(),
                    cosmic::widget::space().width(Length::Fill).into(),
                ])
                .padding(cosmic::iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_xxs,
                ]))
                .height(Length::Fixed(64.0))
                .align_y(Vertical::Center),
                cosmic::widget::divider::horizontal::default(),
            ]),
            // Display on hover, where the controls should be
            // !!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!
            cosmic::widget::container(cosmic::widget::column![
                cosmic::widget::divider::horizontal::light(),
                cosmic::widget::row::with_children(vec![
                    widget::column![
                        cosmic::widget::text::heading(self.title.to_string())
                            .width(Length::FillPortion(1))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                        cosmic::widget::text::text(self.artist.to_string())
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                            .width(Length::FillPortion(1)),
                        cosmic::widget::text::text(self.album_title.to_string())
                            .width(Length::FillPortion(1))
                            .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1))),
                    ]
                    .into(),
                    cosmic::widget::row::with_children(vec![
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("playlist-symbolic"),
                            ))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Add to playlist")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("list-add-symbolic"),
                            ))
                            .on_press(Message::AddTrackById(self.id))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Add to queue")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                        cosmic::iced::widget::tooltip(
                            cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                cosmic::widget::icon::from_name("media-playback-start-symbolic"),
                            ))
                            .on_press(Message::PlayTrackById(self.id))
                            .class(cosmic::theme::Button::Standard),
                            cosmic::widget::container("Play now")
                                .padding(cosmic::theme::spacing().space_xxxs)
                                .class(cosmic::theme::Container::Tooltip),
                            cosmic::widget::tooltip::Position::Top,
                        )
                        .into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_xxxs)
                    .into(), //right
                ])
                .spacing(cosmic::theme::spacing().space_s)
                .padding(cosmic::iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_xxs,
                ]))
                .height(Length::Fixed(64.0))
                .align_y(Vertical::Center),
                cosmic::widget::divider::horizontal::light(),
            ])
            .class(cosmic::theme::Container::Card),
        )])
        .into()
    }

    pub fn to_queued_track(&self) -> QueuedTrack {
        QueuedTrack {
            id: self.id,
            title: self.title.clone(),
            path_buf: self.path_buf.clone(),
        }
    }
}
