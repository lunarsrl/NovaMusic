use std::arch::x86_64::_mm_stream_sd;
use std::ffi::OsStr;
use std::io;
use std::path::PathBuf;
use std::task::Poll;
use cosmic::iced::futures::channel::mpsc::Sender;
use futures_util::SinkExt;
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

    let mut viewed_files: u32 = 0;
    read_dir(path, tx, &mut files, &mut viewed_files).await;
    tx.send(Message::UpdateScanDirSize(files.len() as u32)).await.unwrap();
    files
}

async fn read_dir(path: PathBuf, tx: &mut Sender<Message>, files: &mut Vec<MediaFileTypes>, index: &mut u32){
    match path.read_dir() {
        Ok(dir) => {
            log::info!("Current Directory: {}", path.display());
            for entry in dir {
                match entry {
                    Ok(dir) => {
                        match dir.metadata().unwrap().is_dir() {
                            true => {
                                let found_path = dir.path();
                                log::info!("Found Next Directory: {}", found_path.display());
                                Box::pin(read_dir(found_path, tx, files, index)).await;

                            }
                            false => {
                                log::info!("Found A File: {}", dir.path().display());

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
    match path.extension() {
        None => {
            None
        }
        Some(extension) => {
            match extension.to_str().unwrap().to_lowercase().as_str() {
                "mp4" => {
                    log::info!("--- It's a mp4!");
                    Some(MediaFileTypes::MP4(path))
                }
                "mp3" => {
                    log::info!("--- It's a mp3!");
                    Some(MediaFileTypes::MP3(path))
                }
                "flac" =>{
                    log::info!("--- It's a flac!");
                    Some(MediaFileTypes::FLAC(path))
                }
                _ => {
                    log::info!("--- Unknown/unsupported filetype :(");
                    None
                }
            }
        }

    }



}


