// SPDX-FileCopyrightText: © 2024 Jade Meskill <jade.meskill@gmail.com>
//
// SPDX-License-Identifier: MIT

use crate::client::MatrixClient;
use anyhow::Result;
use clap::Subcommand;
use matrix_sdk::{config::SyncSettings, LoopCtrl};
use tracing::info;

#[derive(Debug, Subcommand)]
pub enum SyncCommands {
    /// Start syncing and listening for events
    Start {
        /// Full sync from scratch
        #[arg(long)]
        full: bool,
    },

    /// Show sync status
    Status,
}

pub async fn handle_sync_command(client: &MatrixClient, command: &SyncCommands) -> Result<()> {
    let inner_client = client.inner_client();
    match command {
        SyncCommands::Start { full } => {
            let settings = if *full {
                SyncSettings::default()
            } else {
                SyncSettings::default()
                // SyncSettings::default().token(inner_client.sync_token().await)
            };

            println!("Starting sync...");
            println!("Press Ctrl+C to stop");

            inner_client
                .sync_with_callback(settings, |response| async move {
                    if !response.rooms.join.is_empty() {
                        info!("Received {} joined room updates", response.rooms.join.len());
                    }
                    if !response.rooms.invite.is_empty() {
                        info!("Received {} room invites", response.rooms.invite.len());
                    }
                    if !response.rooms.leave.is_empty() {
                        info!("Received {} left rooms", response.rooms.leave.len());
                    }
                    LoopCtrl::Continue
                })
                .await?;
        }
        SyncCommands::Status => {
            // if let Some(token) = inner_client.sync_token().await {
            //     println!("Current sync token: {}", token);
            // } else {
            //     println!("No sync token available");
            // }

            println!("Connected to homeserver: {}", inner_client.homeserver());

            if let Some(user_id) = inner_client.user_id() {
                println!("Logged in as: {}", user_id);
            }

            let rooms = inner_client.joined_rooms();
            println!("Joined rooms: {}", rooms.len());
        }
    }
    Ok(())
}
