use std::sync::Arc;
use crate::app;
use crate::app::{AppTrack, Message};
use cosmic::iced::{Alignment, ContentFit, Element, Length};
use cosmic::widget::dropdown::multi::list;
use std::num::Wrapping;

#[derive(Debug, Clone)]
pub struct TrackPage {
    pub tracks: Arc<Vec<AppTrack>>,
    pub track_page_state: TrackPageState,
}

#[derive(Debug, Clone)]
pub enum TrackPageState {
    Loading,
    Loaded,
}

impl TrackPage {
    pub fn new() -> Self {
        TrackPage {
            tracks: Arc::from(Vec::<AppTrack>::new()),
            track_page_state: TrackPageState::Loading,
        }
    }
    pub fn load<'a>(&'a self, model: &'a app::AppModel) -> cosmic::Element<app::Message> {
        cosmic::widget::container::Container::new(
            cosmic::widget::column::with_children(vec![
                cosmic::widget::row::with_children(vec![
                    // HEADING AREA
                    cosmic::widget::row::with_children(vec![
                        cosmic::widget::text::title2("All Tracks")
                            .width(Length::FillPortion(2))
                            .into(),
                        cosmic::widget::horizontal_space()
                            .width(Length::Shrink)
                            .into(),
                        cosmic::widget::search_input("Enter Track Title", &model.search_field)
                            .on_input(|a| Message::UpdateSearch(a))
                            .width(Length::FillPortion(1))
                            .into(),
                    ])
                    .align_y(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_s)
                    .into(),
                ])
                .into(),
                cosmic::widget::scrollable::vertical(track_list_display(&self.tracks)).into(),
            ])
            .spacing(cosmic::theme::spacing().space_m),
        )
        .width(Length::Fill)
        .height(Length::Fill)
        .padding(cosmic::iced_core::Padding::from([
            0,
            cosmic::theme::spacing().space_m,
        ]))
        .into()
    }
}

fn track_list_display(tracks: &Vec<AppTrack>) -> cosmic::Element<'static, app::Message> {
    let mut list_widget = Some(cosmic::widget::ListColumn::new());

    for track in tracks {
        //todo if track is associated with an album, display album cover. Dont know how to do this efficiently yet.
        match list_widget.take() {
            Some(prev_list) => {
                list_widget = Some(
                    // ----CONTENT---- //
                    prev_list.add(cosmic::widget::container::Container::new(
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::text::heading(format!("{}", track.title,))
                                .width(Length::FillPortion(1))
                                .into(),
                            cosmic::widget::text::text(format!("{}", track.artist,))
                                .width(Length::FillPortion(1))
                                .into(),
                            cosmic::widget::text::text(format!("{}", track.album_title,))
                                .width(Length::FillPortion(1))
                                .into(),
                            cosmic::widget::button::icon(cosmic::widget::icon::from_name(
                                "media-playback-start-symbolic",
                            ))
                            .on_press(Message::AddTrackToQueue(
                                track.path_buf.to_string_lossy().to_string(),
                            ))
                            .into(),
                        ])
                        .spacing(cosmic::theme::spacing().space_xxxs)
                        .align_y(Alignment::Center)

                    )),
                )
            }
            None => {}
        }
    }
    list_widget.unwrap().into_element()
}
