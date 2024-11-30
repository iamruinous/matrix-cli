// SPDX-FileCopyrightText: © 2024 Jade Meskill <jade.meskill@gmail.com>
//
// SPDX-License-Identifier: MIT

use anyhow::Result;
use matrix_sdk::{
    config::SyncSettings, matrix_auth::MatrixSession, ruma::api::client::filter::FilterDefinition,
    Client,
};
use rand::{distributions::Alphanumeric, thread_rng, Rng};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tracing::info;

/// The data needed to re-build a client.
#[derive(Debug, Serialize, Deserialize)]
struct ClientSession {
    /// The URL of the homeserver of the user.
    homeserver: String,

    /// The path of the database.
    store_path: PathBuf,

    /// The passphrase of the database.
    passphrase: String,
}

/// The full session to persist.
#[derive(Debug, Serialize, Deserialize)]
struct FullSession {
    /// The data to re-build the client.
    client_session: ClientSession,

    /// The Matrix user session.
    user_session: MatrixSession,

    /// The latest sync token.
    ///
    /// It is only needed to persist it when using `Client::sync_once()` and we
    /// want to make our syncs faster by not receiving all the initial sync
    /// again.
    #[serde(skip_serializing_if = "Option::is_none")]
    sync_token: Option<String>,
}

pub struct MatrixClient {
    client: Client,
    store_path: PathBuf,
    passphrase: String,
    pub session_file: PathBuf,
    pub sync_token: Option<String>,
}

impl MatrixClient {
    pub async fn new(homeserver: &str) -> Result<Self> {
        let store_path = dirs::data_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("matrix-cli");

        // Ensure the directory exists
        tokio::fs::create_dir_all(&store_path).await?;

        let session_file = store_path.clone().join("session.json");

        let (client, sync_token, passphrase) = if session_file.exists() {
            let serialized_session = tokio::fs::read_to_string(&session_file).await?;
            let FullSession {
                client_session,
                user_session,
                sync_token,
            } = serde_json::from_str(&serialized_session)?;

            let client = build_client(homeserver, &store_path, &client_session.passphrase).await?;

            // Restore the Matrix user session.
            client.restore_session(user_session).await?;

            (client, sync_token, client_session.passphrase)
        } else {
            let mut rng = thread_rng();
            // Generate a random passphrase.

            let passphrase = (&mut rng)
                .sample_iter(Alphanumeric)
                .take(32)
                .map(char::from)
                .collect::<String>();

            let client = build_client(homeserver, &store_path, &passphrase).await?;
            (client, None, passphrase)
        };

        Ok(Self {
            client,
            store_path,
            passphrase,
            session_file,
            sync_token,
        })
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<()> {
        // Check if we already have a valid session
        if self.client.logged_in() {
            info!("Using existing session");
            return Ok(());
        }

        info!("Logging in as {}", username);
        self.client
            .matrix_auth()
            .login_username(username, password)
            .initial_device_display_name("matrix-cli")
            .await?;

        info!("Login successful");
        self.save_session().await?;
        Ok(())
    }

    pub async fn logout(&self) -> Result<()> {
        self.client.matrix_auth().logout().await?;

        // Clear the state store
        tokio::fs::remove_dir_all(&self.store_path).await?;
        tokio::fs::create_dir_all(&self.store_path).await?;

        println!("Logged out and cleared session data");
        Ok(())
    }

    pub async fn sync_once(&self) -> Result<()> {
        if !self.client.logged_in() {
            info!("Not logged in, skipping sync_once");
            return Ok(());
        }

        // Enable room members lazy-loading, it will speed up the initial sync a lot
        // with accounts in lots of rooms.
        // See <https://spec.matrix.org/v1.6/client-server-api/#lazy-loading-room-members>.
        let filter = FilterDefinition::with_lazy_loading();

        let mut sync_settings = SyncSettings::default().filter(filter.into());

        // We restore the sync where we left.
        // This is not necessary when not using `sync_once`. The other sync methods get
        // the sync token from the store.
        info!(?self.sync_token, "Initial sync token");
        if let Some(sync_token) = &self.sync_token {
            sync_settings = sync_settings.token(sync_token);
        }

        // Let's ignore messages before the program was launched.
        // This is a loop in case the initial sync is longer than our timeout. The
        // server should cache the response and it will ultimately take less time to
        // receive.
        loop {
            match self.client.sync_once(sync_settings.clone()).await {
                Ok(response) => {
                    persist_sync_token(self.session_file.clone(), response.next_batch).await?;
                    break;
                }
                Err(error) => {
                    println!("An error occurred during initial sync: {error}");
                    println!("Trying again…");
                }
            }
        }
        info!("sync_once complete");
        Ok(())
    }

    // pub async fn sync(&self) -> Result<()> {
    //     if !self.client.logged_in() {
    //         info!("Not logged in, skipping sync_once");
    //         return Ok(());
    //     }

    //     // Enable room members lazy-loading, it will speed up the initial sync a lot
    //     // with accounts in lots of rooms.
    //     // See <https://spec.matrix.org/v1.6/client-server-api/#lazy-loading-room-members>.
    //     let filter = FilterDefinition::with_lazy_loading();

    //     let mut sync_settings = SyncSettings::default().filter(filter.into());

    //     // We restore the sync where we left.
    //     // This is not necessary when not using `sync_once`. The other sync methods get
    //     // the sync token from the store.
    //     info!(?self.sync_token, "Initial sync token");
    //     if let Some(sync_token) = &self.sync_token {
    //         sync_settings = sync_settings.token(sync_token);
    //     }

    //     let session_file = self.session_file.as_path();

    //     // This loops until we kill the program or an error happens.
    //     self.client
    //         .sync_with_result_callback(sync_settings, |sync_result| async move {
    //             let response = sync_result?;

    //             // We persist the token each time to be able to restore our session
    //             persist_sync_token(session_file.to_path_buf(), response.next_batch)
    //                 .await
    //                 .map_err(|err| Error::UnknownError(err.into()))?;

    //             Ok(LoopCtrl::Continue)
    //         })
    //         .await?;

    //     info!("sync complete");
    //     Ok(())
    // }

    pub async fn save_session(&self) -> Result<()> {
        let client_session = ClientSession {
            homeserver: self.client.homeserver().to_string(),
            store_path: self.store_path.clone(),
            passphrase: self.passphrase.clone(),
        };
        let user_session = self
            .client
            .matrix_auth()
            .session()
            .expect("A logged-in client should have a session");
        let serialized_session = serde_json::to_string(&FullSession {
            client_session,
            user_session,
            sync_token: None,
        })?;
        tokio::fs::write(&self.session_file, serialized_session).await?;

        info!("Session saved successfully");
        Ok(())
    }

    // Getter for the underlying client, so command handlers can use it
    pub fn inner_client(&self) -> &Client {
        &self.client
    }
}

async fn build_client(
    homeserver: &str,
    store_path: &Path,
    passphrase: &str,
) -> anyhow::Result<Client> {
    let client_builder = Client::builder()
        .homeserver_url(homeserver)
        .sqlite_store(store_path, Some(passphrase))
        .user_agent("matrix-cli/v0.1.0");

    Ok(client_builder.build().await?)
}

/// Persist the sync token for a future session.
/// Note that this is needed only when using `sync_once`. Other sync methods get
/// the sync token from the store.
async fn persist_sync_token(session_file: PathBuf, sync_token: String) -> anyhow::Result<()> {
    let serialized_session = tokio::fs::read_to_string(&session_file).await?;
    let mut full_session: FullSession = serde_json::from_str(&serialized_session)?;

    full_session.sync_token = Some(sync_token.clone());
    let serialized_session = serde_json::to_string(&full_session)?;
    tokio::fs::write(&session_file, serialized_session).await?;

    Ok(())
}
