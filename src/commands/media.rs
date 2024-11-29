// SPDX-FileCopyrightText: © 2024 Jade Meskill <jade.meskill@gmail.com>
//
// SPDX-License-Identifier: MIT

use crate::client::MatrixClient;
use anyhow::Result;
use clap::Subcommand;
use matrix_sdk::{attachment::AttachmentConfig, ruma::RoomId};
use std::path::PathBuf;

#[derive(Debug, Subcommand)]
pub enum MediaCommands {
    /// Upload and send media to a room
    Upload {
        /// Room ID
        room_id: String,
        /// Path to media file
        file_path: PathBuf,
        /// Caption (optional)
        #[arg(long)]
        caption: Option<String>,
    },

    /// Download media from a room
    Download {
        /// Room ID
        room_id: String,
        /// Media event ID
        event_id: String,
        /// Output directory
        #[arg(long, default_value = ".")]
        output_dir: PathBuf,
    },
}

pub async fn handle_media_command(client: &MatrixClient, command: &MediaCommands) -> Result<()> {
    let inner_client = client.inner_client();
    match command {
        MediaCommands::Upload {
            room_id,
            file_path,
            caption,
        } => {
            let room_id = RoomId::parse(room_id)?;
            if let Some(room) = inner_client.get_room(&room_id) {
                let content = tokio::fs::read(file_path).await?;
                let mime_type = mime_guess::from_path(file_path).first_or_octet_stream();

                println!("Uploading media...");

                let attachment_config = AttachmentConfig::new().caption(caption.clone());
                let event_id = room
                    .send_attachment(
                        file_path.file_name().unwrap().to_str().unwrap(),
                        &mime_type,
                        content,
                        attachment_config,
                    )
                    .await?;

                if room.is_encrypted().await? {
                    println!(
                        "Media uploaded and sent encrypted (event_id: {:?})",
                        event_id
                    );
                } else {
                    println!("Media uploaded and sent (event_id: {:?})", event_id);
                }
            }
        }
        MediaCommands::Download {
            room_id,
            event_id,
            output_dir,
        } => {
            // client.download_media(room_id, event_id, output_dir).await?;
            println!("Not implemented {} {} {:?}", room_id, event_id, output_dir);
        }
    }
    Ok(())
}
