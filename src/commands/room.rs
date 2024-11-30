// SPDX-FileCopyrightText: © 2024 Jade Meskill <jade.meskill@gmail.com>
//
// SPDX-License-Identifier: MIT

use std::path::PathBuf;

use crate::client::MatrixClient;
use anyhow::Result;
use clap::Subcommand;
use matrix_sdk::ruma::api::client::room::create_room::v3::Request as CreateRoomRequest;
use matrix_sdk::ruma::api::client::room::create_room::v3::RoomPreset;
use matrix_sdk::ruma::api::client::room::Visibility;
use matrix_sdk::ruma::RoomId;
use matrix_sdk::ruma::RoomOrAliasId;
use matrix_sdk::ruma::UserId;
use tracing::info;

#[derive(Debug, Subcommand)]
pub enum RoomCommands {
    /// Join a room
    Join {
        /// Room ID or alias
        room_id: String,
    },

    /// Leave a room
    Leave {
        /// Room ID
        room_id: String,
    },

    /// List joined rooms
    List,

    /// Create a new room
    Create {
        /// Room name
        name: String,
        /// Room topic (optional)
        #[arg(long)]
        topic: Option<String>,
        /// Make room public (default is private)
        #[arg(long)]
        public: bool,
    },

    /// Invite a user to a room
    Invite {
        /// Room ID
        room_id: String,
        /// User ID to invite
        user_id: String,
    },
    /// Upload and set an avatar from a local file
    UploadAvatar {
        /// Room ID
        room_id: String,
        /// Path to the image file to upload
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
}

pub async fn handle_room_command(client: &MatrixClient, command: &RoomCommands) -> Result<()> {
    let inner_client = client.inner_client();
    match command {
        RoomCommands::Join { room_id } => {
            let room_alias = <&RoomOrAliasId>::try_from(room_id.as_str()).unwrap();
            let server_name = room_alias.server_name().unwrap().to_owned();
            let _room = inner_client
                .join_room_by_id_or_alias(room_alias, &[server_name])
                .await?;
            println!("Joined room: {}", room_id);
        }
        RoomCommands::Leave { room_id } => {
            let room_alias = <&RoomId>::try_from(room_id.as_str()).unwrap();
            let room = inner_client
                .get_room(room_alias)
                .ok_or_else(|| anyhow::anyhow!("Room not found"))?;
            room.leave().await?;
            println!("Left room: {}", room_id);
        }
        RoomCommands::List => {
            let joined_rooms = inner_client.joined_rooms();
            println!("Joined rooms:");
            for room in joined_rooms {
                let name = room.name().unwrap_or_else(|| room.room_id().to_string());
                println!("- {} ({})", name, room.room_id());
            }
        }
        RoomCommands::Create {
            name,
            topic,
            public,
        } => {
            let mut room_request = CreateRoomRequest::new();
            room_request.name = Some(name.clone());
            room_request.topic.clone_from(topic);
            room_request.visibility = if *public {
                Visibility::Public
            } else {
                Visibility::Private
            };
            room_request.preset = Some(RoomPreset::PublicChat);
            let room = inner_client.create_room(room_request).await?;
            println!("Created room: {} ({})", name, room.room_id());
        }
        RoomCommands::Invite { room_id, user_id } => {
            let room_alias = <&RoomId>::try_from(room_id.as_str()).unwrap();

            let room = inner_client
                .get_room(room_alias)
                .ok_or_else(|| anyhow::anyhow!("Room not found"))?;
            let user_alias = <&UserId>::try_from(user_id.as_str()).unwrap();
            room.invite_user_by_id(user_alias).await?;
            println!("Invited {} to room {}", user_id, room_id);
        }
        RoomCommands::UploadAvatar { room_id, path } => {
            let room_alias = <&RoomOrAliasId>::try_from(room_id.as_str()).unwrap();
            let server_name = room_alias.server_name().unwrap().to_owned();
            let room = inner_client
                .join_room_by_id_or_alias(room_alias, &[server_name])
                .await?;

            let guess = mime_guess::from_path(path);
            let content = std::fs::read(path)?;
            room.upload_avatar(&guess.first().unwrap(), content, None)
                .await?;
            info!("Avatar successfully uploaded and set");
            println!("Set avatar for room {}", room_id);
        }
    }
    Ok(())
}
