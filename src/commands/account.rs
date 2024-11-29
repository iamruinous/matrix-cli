use std::path::PathBuf;

use anyhow::Result;
use clap::Subcommand;
use matrix_sdk::{media::MediaFormat, ruma::MxcUri};
use tracing::info;

use crate::client::MatrixClient;

#[derive(Debug, Subcommand)]
pub enum AccountCommands {
    /// Set your avatar URL
    SetAvatarUrl {
        /// The MXC URL of the avatar image (must start with mxc://)
        #[arg(value_name = "URL")]
        url: String,
    },
    /// Unset your avatar URL
    UnsetAvatarUrl,
    /// Get your current avatar URL
    GetAvatarUrl,
    /// Upload and set an avatar from a local file
    UploadAvatar {
        /// Path to the image file to upload
        #[arg(value_name = "PATH")]
        path: PathBuf,
    },
    /// Download your current avatar to a file
    DownloadAvatar {
        /// The path where the avatar should be saved
        #[arg(value_name = "PATH")]
        output_path: PathBuf,
    },
    /// Get your current display name
    GetDisplayName,
}

pub async fn handle_account_command(
    client: &MatrixClient,
    command: &AccountCommands,
) -> Result<()> {
    match command {
        AccountCommands::SetAvatarUrl { url } => {
            if !url.starts_with("mxc://") {
                anyhow::bail!("Avatar URL must start with mxc://");
            }

            let content_uri = Box::<MxcUri>::from(&url[..]);
            client
                .inner_client()
                .account()
                .set_avatar_url(Some(&content_uri))
                .await?;
            info!("Avatar URL successfully updated");
        }
        AccountCommands::UnsetAvatarUrl {} => {
            client.inner_client().account().set_avatar_url(None).await?;
            info!("Avatar URL successfully updated");
        }
        AccountCommands::GetAvatarUrl => {
            let avatar_url = client.inner_client().account().get_avatar_url().await?;
            match avatar_url {
                Some(url) => println!("Current avatar URL: {}", url),
                None => println!("No avatar URL set"),
            }
        }
        AccountCommands::DownloadAvatar { output_path } => {
            if let Some(avatar) = client
                .inner_client()
                .account()
                .get_avatar(MediaFormat::File)
                .await?
            {
                std::fs::write(output_path, avatar)?;
                info!(
                    "Avatar downloaded successfully to {}",
                    output_path.display()
                );
            }
        }
        AccountCommands::UploadAvatar { path } => {
            let guess = mime_guess::from_path(path);
            let content = std::fs::read(path)?;
            client
                .inner_client()
                .account()
                .upload_avatar(&guess.first().unwrap(), content)
                .await?;
            info!("Avatar successfully uploaded and set");
        }
        AccountCommands::GetDisplayName => {
            let display_name = client.inner_client().account().get_display_name().await?;
            match display_name {
                Some(name) => println!("Current display name: {}", name),
                None => println!("No display name set"),
            }
        }
    }
    Ok(())
}
