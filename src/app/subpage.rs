use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::iced::advanced::text::{Ellipsize, EllipsizeHeightLimit};
use cosmic::iced::Length;
use cosmic::Element;

pub trait Subpage {
    fn header_title(&self) -> String;

    fn header_image(&self) -> Element<Message>;
    fn header_subtitle(&self) -> String;
    fn body(&self, model: &AppModel) -> Element<Message>;
}

pub trait SubpageBuilder {
    fn page(&self, model: &AppModel) -> Element<Message>;
    fn header(&self) -> Element<Message>;
}

impl<T: Subpage> SubpageBuilder for T {
    fn header(&self) -> Element<Message> {
        cosmic::widget::column::with_children(vec![
            cosmic::widget::row::with_children(vec![cosmic::widget::button::custom(
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::icon::from_name("go-previous-symbolic").into(),
                    cosmic::widget::text::text(fl!("artists")).into(),
                ])
                .align_y(cosmic::iced::Alignment::Center),
            )
            .on_press(Message::Return)
            .class(cosmic::widget::button::ButtonClass::Link)
            .into()])
            .into(),
            cosmic::widget::row::with_children(vec![
                self.header_image(),
                cosmic::widget::column::with_children(vec![
                    cosmic::widget::text::title2(self.header_title())
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(2)))
                        .into(),
                    cosmic::widget::text::title3(self.header_subtitle())
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(1)))
                        .into(),
                    cosmic::widget::space::vertical().into(),
                    cosmic::widget::column::with_children(vec![
                        cosmic::widget::divider::horizontal::default().into(),
                        cosmic::widget::row::with_children(vec![
                            cosmic::iced::widget::tooltip(
                                cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                    cosmic::widget::icon::from_name(
                                        "media-playback-start-symbolic",
                                    ),
                                ))
                                .class(cosmic::theme::Button::Standard),
                                cosmic::widget::container(cosmic::widget::text(fl!("PlayNow")))
                                    .padding(cosmic::theme::spacing().space_xxxs)
                                    .class(cosmic::theme::Container::Tooltip),
                                cosmic::widget::tooltip::Position::Top,
                            )
                            .into(),
                            cosmic::iced::widget::tooltip(
                                cosmic::widget::button::icon(cosmic::widget::icon::Handle::from(
                                    cosmic::widget::icon::from_name("playlist-symbolic"),
                                ))
                                .class(cosmic::theme::Button::Standard),
                                cosmic::widget::container(cosmic::widget::text(fl!("AddToQueue")))
                                    .padding(cosmic::theme::spacing().space_xxxs)
                                    .class(cosmic::theme::Container::Tooltip),
                                cosmic::widget::tooltip::Position::Top,
                            )
                            .into(),
                        ])
                        .align_y(cosmic::iced::Alignment::Center)
                        .spacing(cosmic::theme::spacing().space_xxs)
                        .into(),
                        cosmic::widget::divider::horizontal::default().into(),
                    ])
                    .spacing(cosmic::theme::spacing().space_xxs)
                    .into(),
                ])
                .into(),
            ])
            .spacing(cosmic::theme::spacing().space_s)
            .into(),
        ])
        .height(Length::Fixed(220.0))
        .spacing(cosmic::theme::spacing().space_s)
        .into()
    }

    fn page(&self, model: &AppModel) -> Element<Message> {
        cosmic::widget::scrollable::vertical(
            cosmic::widget::column::with_children(vec![self.header(), self.body(model)])
                .spacing(cosmic::theme::spacing().space_s)
                .padding(cosmic::iced::core::padding::Padding::from([
                    0,
                    cosmic::theme::spacing().space_s,
                ])),
        )
        .height(Length::Fill)
        .into()
    }
}
