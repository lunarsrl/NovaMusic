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
                    cosmic::widget::text::title3(self.header_title())
                        .ellipsize(Ellipsize::End(EllipsizeHeightLimit::Lines(2)))
                        .into(),
                    cosmic::widget::text::title4(self.header_subtitle()).into(),
                    cosmic::widget::space::vertical().into(),
                    cosmic::widget::column::with_children(vec![
                        cosmic::widget::divider::horizontal::default().into(),
                        cosmic::widget::row::with_children(vec![
                            cosmic::widget::button::text(fl!("AddToQueue"))
                                .class(cosmic::widget::button::ButtonClass::Suggested)
                                .into(),
                            cosmic::widget::button::text(fl!("ReplaceQueue"))
                                .class(cosmic::widget::button::ButtonClass::Suggested)
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
                .align_x(cosmic::iced::Alignment::Start)
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
            cosmic::widget::column::with_children(vec![self.header(), self.body(model)]).padding(
                cosmic::iced::core::padding::Padding::from([0, cosmic::theme::spacing().space_s]),
            ),
        )
        .into()
    }
}
