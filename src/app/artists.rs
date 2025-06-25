use std::future::IntoFuture;
use cosmic::Element;
use cosmic::iced_runtime::task::widget;
use cosmic::prelude::CollectionWidget;
use cosmic::widget::Widget;
use crate::app;
use crate::app::{AppModel};

struct ArtistInfo {
    name: String,
    image: String,
}

enum TopLevelInfo {
    Albums,
    Artists,
    Playlists,
    NowPlaying,
}
impl AppModel {
    pub fn grid(&self, info: TopLevelInfo) -> Element<app::Message> {
        match  info {
            TopLevelInfo::Albums => {
                
            }
            TopLevelInfo::Artists => {}
            TopLevelInfo::Playlists => {}
            TopLevelInfo::NowPlaying => {}
        }
        cosmic::widget::flex_row(vec![
            

            
        ]).into()
    }
}

pub fn artist_box() -> Element<'static, app::Message>{
   cosmic::widget::Column::new().push(
       cosmic::widget::Text::new("IMAGE")
   ).push(
       cosmic::widget::Text::new("Artist Box")
   )
       .into()
}


