use crate::app::page::albums::FullAlbum;
use crate::app::{AppModel, Message};
use crate::fl;
use cosmic::Element;

pub trait Subpage {
    fn title(&self) -> String;
    fn body(&self, model: &AppModel) -> Element<Message>;
}

trait SubPageBuilder {
    fn page(&self, model: &AppModel) -> Element<Message>;
    fn header(&self) -> Element<Message>;
}

impl<T: Subpage> SubPageBuilder for T {
    fn header(&self) -> Element<Message> {}

    fn page(&self, model: &AppModel) -> Element<Message> {
        todo!()
    }
}

impl Subpage for FullAlbum {
    fn title(&self) -> String {
        self.title()
    }

    fn body(&self, model: &AppModel) -> Element<Message> {
        cosmic::widget::column::with_children(vec![cosmic::widget::row::with_children(vec![
            cosmic::widget::button::custom(
                cosmic::widget::row::with_children(vec![
                    cosmic::widget::icon::from_name("go-previous-symbolic").into(),
                    cosmic::widget::text::text(fl!("artists")).into(),
                ])
                .align_y(cosmic::iced::Alignment::Center),
            )
            .on_press(Message::ArtistPageReturn)
            .class(cosmic::widget::button::ButtonClass::Link)
            .into(),
        ])
        .into()])
        .into()
    }
}
