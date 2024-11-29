// SPDX-FileCopyrightText: © 2024 Jade Meskill <jade.meskill@gmail.com>
//
// SPDX-License-Identifier: MIT

use crate::client::MatrixClient;
use anyhow::Result;
use clap::Subcommand;
use matrix_sdk::ruma::{events::room::message::RoomMessageEventContent, RoomId};

#[derive(Debug, Subcommand)]
pub enum MessageCommands {
    /// Send a text message
    Send {
        /// Room ID
        room_id: String,
        /// Message text
        message: String,
    },

    /// Reply to a message
    Reply {
        /// Room ID
        room_id: String,
        /// Message to reply to (event ID)
        reply_to: String,
        /// Reply message text
        message: String,
    },

    /// React to a message
    React {
        /// Room ID
        room_id: String,
        /// Message to react to (event ID)
        event_id: String,
        /// Reaction emoji
        emoji: String,
    },

    /// List recent messages in a room
    List {
        /// Room ID
        room_id: String,
        /// Number of messages to fetch
        #[arg(default_value = "30")]
        limit: u32,
    },
}

pub async fn handle_message_command(
    client: &MatrixClient,
    command: &MessageCommands,
) -> Result<()> {
    let inner_client = client.inner_client();
    match command {
        MessageCommands::Send { room_id, message } => {
            let room_id = RoomId::parse(room_id)?;
            if let Some(room) = inner_client.get_room(&room_id) {
                let content = RoomMessageEventContent::text_plain(message);
                let event_id = room.send(content).await?;

                if room.is_encrypted().await? {
                    println!("Message sent encrypted (event_id: {:?})", event_id);
                } else {
                    println!("Message sent (event_id: {:?})", event_id);
                }
            }
        }
        MessageCommands::Reply {
            room_id,
            reply_to,
            message,
        } => {
            // let room_id = RoomId::parse(room_id)?;
            // if let Some(room) = inner_client.get_room(&room_id) {
            //     let content = RoomMessageEventContent::text_plain(message)
            //     .make_reply_to(
            //         reply_to,
            //     );
            //     let event_id = room.send(content).await?;
            //     println!("Reply sent successfully (event_id: {:?})", event_id);
            // }
            println!("Not implemented {} {} {}", room_id, reply_to, message);
        }
        MessageCommands::React {
            room_id,
            event_id,
            emoji,
        } => {
            // inner_client.react_to_message(room_id, event_id, emoji).await?;
            println!("Not implemented {} {} {}", room_id, event_id, emoji);
        }
        MessageCommands::List { room_id, limit } => {
            // let room_id = RoomId::parse(room_id)?;
            // if let Some(room) = inner_client.get_room(&room_id) {
            //     let mut options = MessagesOptions::backward().from("t47429-4392820_219380_26003_2265");
            //     options.limit = Some(limit as UInt);
            //     let messages = room.messages(options)
            //         .await?;

            //     println!("Recent messages in {}:", room_id);
            //     for msg in messages.chunk {
            //         if let Some(content) = msg.content.as_message() {
            //             let sender = msg.sender;
            //             let body = content.body();
            //             let encrypted = if room.is_encrypted() { "[encrypted] " } else { "" };
            //             println!("{}{}: {}", encrypted, sender, body);
            //         }
            //     }
            // }
            println!("Not implemented {} {}", room_id, limit);
        }
    }
    Ok(())
}
