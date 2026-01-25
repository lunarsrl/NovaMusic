// SPDX-License-Identifier: GPL-2.0-or-later

use crate::app::Message;
use cosmic::iced::futures::channel::mpsc::Sender;
use futures_util::SinkExt;
use std::path::PathBuf;

pub async fn scan_directory(path: PathBuf, tx: &mut Sender<Message>) {
    let mut index = 0;
    read_dir(path, tx, &mut index).await
}

async fn read_dir(path: PathBuf, tx: &mut Sender<Message>, index: &mut u32) {
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
        let _ = tx
            .send(Message::ToastError(String::from(format!(
                "Error at path: {}",
                path.to_string_lossy().to_string()
            ))))
            .await;
    }
}
