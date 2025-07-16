use cosmic::iced::{Alignment, Element, Length};

use crate::app;

#[derive(Debug)]
pub struct TrackPage;
impl TrackPage {
    pub fn load(&self) -> cosmic::Element<'static, app::Message>{
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
                     cosmic::widget::search_input("Enter Track Title", "").width(Length::FillPortion(1)).into(),
                ])
                    .align_y(Alignment::Center)
                    .spacing(cosmic::theme::spacing().space_s)
                    .into(),

            ]).into(),
                track_list_display()
            ])

        )
            .width(Length::Fill)
            .height(Length::Fill)
            .padding(cosmic::iced_core::Padding::from([0, cosmic::theme::spacing().space_m]))
            .into()
    }
}

fn track_list_display() -> cosmic::Element<'static, app::Message>{
   cosmic::widget::list_column().into_element()
}