use cosmic::Element;
use cosmic::iced::alignment::Horizontal;
use cosmic::iced_runtime::task::widget;
use crate::app;


struct Album {
    name: String,
    artist: String,
    disc_number: u32,
    track_number: u32,
    
    
    //for getting visual prob make async
    path: String,
}

pub fn top_album_page() -> Element<'static, app::Message> {
    
   let boxes = get_top_album_info().iter().map(|album| album.grid_box()).collect(); 
    
    cosmic::widget::flex_row(
        
            boxes
        
    ).into()
}

impl Album {
    fn grid_box(&self) -> Element<'static, app::Message> {
        cosmic::widget::Column::with_children(
            [
                
                cosmic::widget::Text::new(self.name.clone()).align_x(Horizontal::Center).into(),
                cosmic::widget::Text::new(self.artist.clone()).align_x(Horizontal::Center).into(),
            ]
        ).into()
    }
}

fn get_top_album_info() -> Vec<Album> {
    let conn = rusqlite::Connection::open("cosmic_music.db").unwrap();


    let row_num = conn.query_row("SELECT COUNT(*) as row_count
    FROM album", (), |row| {
        Ok(row.get::<usize, u32>(0).unwrap())
    }).expect("error");
    
    let mut albums = Vec::new();
    
    for each in 1..=row_num {
        albums.push(
            match conn.query_row("SELECT * FROM album where id = ?", [each], |row| {
            log::info!("{:?}", row);

            let artists_id = row.get::<usize, i32>(2).unwrap();


            let artists_name = conn.query_row("select * from artists where id = ?", [artists_id], |row| {
                match row.get::<usize, String>(1) {
                    Ok(val) => {
                        Ok(val)
                    }
                    Err(_) => {
                        panic!("error")
                    }
                }
                //todo dont make the program crash if metadata is wrong
            }).expect("metadata error");

            Ok(
                Album {
                    name: row.get::<usize, String>(1).unwrap(),
                    artist: artists_name,
                    disc_number: row.get::<usize, u32>(3).unwrap(),
                    track_number: row.get::<usize, u32>(4).unwrap(),
                    path: "".to_string(),
                }
            )
        }) {
                Ok(val) => {
                    val
                }
                Err(_) => {
                    log::info!("EACH: {}", each);
                    panic!(
                        "error"
                    )
                }
            }
        )
    }
    albums
}
