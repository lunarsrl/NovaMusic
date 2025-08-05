use std::arch::x86_64::_mm_stream_sd;
use std::ffi::OsStr;
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::task::Poll;
use cosmic::dialog::file_chooser::open::file;
use cosmic::iced::futures::channel::mpsc::Sender;
use futures_util::SinkExt;
use rusqlite::fallible_iterator::FallibleIterator;
use rust_embed::utils::FileEntry;
use tokio::fs::DirEntry;
use crate::app;
use crate::app::{AppModel, Message};

struct Album {
    tracks: Vec<PathBuf>,
}

#[derive(Debug)]
pub enum MediaFileTypes {
    MP4(PathBuf),
    MP3(PathBuf),
    FLAC(PathBuf),
}


pub async fn scan_directory(path: PathBuf, tx: &mut Sender<Message>) {
    let mut index = 0;
    read_dir(path, tx, &mut index).await
}

async fn read_dir(path: PathBuf, tx: &mut Sender<Message>, index: &mut u32){
    if let Ok(dir) = path.read_dir() {
       for entry in dir {
           if let Ok(entry) = entry {
               let path = entry.path();
              if let Ok(entry) = entry.metadata() {
                  if entry.is_dir() {
                      Box::pin(read_dir(path, tx, index)).await;
                  } else {
                      tx.send(Message::UpdateScanDirSize).await.unwrap();
                      tx.send(Message::AddToDatabase(path.clone())).await.unwrap();

                  }
              }
           }
       }
    } else {
        todo!("error toast")
    }
}
async fn filter_files(path: PathBuf) -> Option<MediaFileTypes> {
    log::info!("Filtering files: {:?}", path);
    match path.extension() {

        None => {
            log::info!("Failed to extract extension");
            None
        }
        Some(extension) => {
            match extension.to_str().unwrap().to_lowercase().as_str() {
                "mp4" => {
                    Some(MediaFileTypes::MP4(path))
                }
                "mp3" => {
                    Some(MediaFileTypes::MP3(path))
                }
                "flac" =>{
                    Some(MediaFileTypes::FLAC(path))
                }
                "m4a" => {
                    Some(MediaFileTypes::MP4(path))
                }
                _ => {
                    None
                }
            }
        }

    }
}

fn read_m3u(mut file_entry: File) {

}


