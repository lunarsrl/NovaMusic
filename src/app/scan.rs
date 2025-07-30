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


pub async fn scan_directory(path: PathBuf, tx: &mut Sender<Message>) -> Vec<MediaFileTypes> {
    let mut files = vec![];

    log::info!("Scanning directory: {:?}", path);
    read_dir(path, tx, &mut files).await;
    tx.send(Message::UpdateScanDirSize(files.len() as u32)).await.unwrap();
    files
}

async fn read_dir(path: PathBuf, tx: &mut Sender<Message>, files: &mut Vec<MediaFileTypes>){
    match path.read_dir() {
        Ok(dir) => {
            for entry in dir {


                match entry {
                    Ok(dir) => {
                        match dir.metadata().unwrap().is_dir() {
                            true => {
                                let found_path = dir.path();
                                Box::pin(read_dir(found_path, tx, files)).await;
                            }
                            false => {

                                match filter_files(dir.path()).await {
                                    Some(dir) => {
                                        files.push(dir);
                                    }
                                    None => {
                                    }
                                }

                            }
                        }
                    }
                    Err(_) => {
                        log::error!("Scan directory could not be opened");
                    }
                }
            }
        }
        Err(err) => {
            log::error!("ERROR: Reading Dir Path in Config{:?}", err);
        }
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


