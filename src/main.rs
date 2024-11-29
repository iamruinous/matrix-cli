// SPDX-FileCopyrightText: © 2024 Jade Meskill <jade.meskill@gmail.com>
//
// SPDX-License-Identifier: MIT

use anyhow::Result;
use clap::{Parser, Subcommand};
use config::{Config, Environment, File};
use serde::Deserialize;
use std::path::PathBuf;
use tracing::{debug, info, instrument, warn};
use tracing_subscriber::{
    fmt::{self, format::FmtSpan},
    prelude::*,
    EnvFilter,
};

mod client;
mod commands;

use client::MatrixClient;
use commands::{media, message, room, sync};

#[derive(Debug, Deserialize)]
struct MatrixConfig {
    homeserver: String,
    username: String,
    password: String,
    // Optional additional config fields
    // device_name: Option<String>,
    // store_path: Option<PathBuf>,
    #[serde(default = "default_log_level")]
    log_level: String,

    #[serde(default)]
    json_logging: bool,

    #[serde(default = "default_log_path")]
    log_path: PathBuf,
}

fn default_log_level() -> String {
    "error".to_string()
}

fn default_log_path() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join("matrix-cli")
        .join("logs")
}

impl MatrixConfig {
    fn load() -> Result<Self> {
        let config_path = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("matrix-cli")
            .join("config.toml");

        debug!(?config_path, "Loading configuration");

        // Create config builder
        let config = Config::builder()
            // Start with default values
            .set_default("device_name", "matrix-cli")?
            .set_default("log_level", "error")?
            .set_default("json_logging", false)?
            // Add config file if it exists
            .add_source(File::from(config_path).required(false))
            // Add config file from custom location if specified
            .add_source(
                Environment::with_prefix("MATRIX")
                    .try_parsing(true)
                    .separator("_"),
            )
            .build()?;

        // Deserialize the config into our struct
        let matrix_config: MatrixConfig = config.try_deserialize()?;

        Ok(matrix_config)
    }

    fn create_default_config() -> Result<()> {
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("matrix-cli");

        std::fs::create_dir_all(&config_dir)?;

        let config_path = config_dir.join("config.toml");
        if !config_path.exists() {
            let default_config = r#"# Matrix CLI Configuration

# Required settings
homeserver = "https://matrix.org"
username = "@user:matrix.org"
password = "your_password"

# Optional settings
device_name = "matrix-cli"
# store_path = "/custom/path/to/store"
# log_level = "info"
# json_logging = false

"#;
            std::fs::write(config_path, default_config)?;
        }
        Ok(())
    }
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Path to config file (optional)
    #[arg(long, env = "MATRIX_CONFIG")]
    config: Option<PathBuf>,

    /// Override log level (error, warn, info, debug, trace)
    #[arg(long, env = "MATRIX_LOG")]
    log_level: Option<String>,

    /// Enable JSON logging
    #[arg(long)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Initialize default configuration file
    Init,

    /// Login and save session
    Login,

    /// Logout and clear session
    Logout,

    /// Room management commands
    #[command(subcommand)]
    Room(room::RoomCommands),

    /// Message commands
    #[command(subcommand)]
    Message(message::MessageCommands),

    /// Media management commands
    #[command(subcommand)]
    Media(media::MediaCommands),

    /// Sync commands
    #[command(subcommand)]
    Sync(sync::SyncCommands),
}

fn setup_logging(
    config_level: &str,
    cli_level: Option<&str>,
    json_logging: bool,
    log_path: &PathBuf,
) -> Result<()> {
    // Create log directory if it doesn't exist
    std::fs::create_dir_all(log_path)?;

    // Set up file appender
    let file_appender = tracing_appender::rolling::daily(log_path, "matrix-cli");

    // Create EnvFilter from log level
    let log_level = cli_level.unwrap_or(config_level);
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        EnvFilter::new(format!(
            "matrix_cli={},matrix_sdk={},warn",
            log_level, log_level
        ))
    });

    // Set up subscribers based on JSON logging preference
    if json_logging {
        // JSON formatter for both console and file
        let formatter = fmt::format()
            .with_level(true)
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .json();

        // Console subscriber
        let console_layer = fmt::layer()
            .event_format(formatter.clone())
            .with_writer(std::io::stdout);

        // File subscriber
        let file_layer = fmt::layer()
            .event_format(formatter)
            .with_writer(file_appender);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    } else {
        // Pretty formatting for console, JSON for file
        let console_layer = fmt::layer()
            .pretty()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_writer(std::io::stdout);

        let file_layer = fmt::layer()
            .json()
            .with_target(true)
            .with_thread_ids(true)
            .with_thread_names(true)
            .with_file(true)
            .with_line_number(true)
            .with_span_events(FmtSpan::NEW | FmtSpan::CLOSE)
            .with_writer(file_appender);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(console_layer)
            .with(file_layer)
            .init();
    }

    debug!(log_level, json_logging, ?log_path, "Logging initialized");

    Ok(())
}

#[tokio::main]
#[instrument]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    // Handle init command before loading config
    if matches!(cli.command, Commands::Init) {
        MatrixConfig::create_default_config()?;
        println!("Created default configuration file");
        return Ok(());
    }

    // Load configuration
    let config = MatrixConfig::load()?;

    // Setup logging with config and CLI options
    setup_logging(
        &config.log_level,
        cli.log_level.as_deref(),
        cli.json || config.json_logging,
        &config.log_path,
    )?;

    info!(
        version = env!("CARGO_PKG_VERSION"),
        "Matrix CLI starting up"
    );

    debug!(
        homeserver = %config.homeserver,
        username = %config.username,
        "Configuration loaded"
    );

    // Initialize client
    let client = MatrixClient::new(&config.homeserver).await?;
    client.sync_once().await?;

    match &cli.command {
        Commands::Init => unreachable!(), // Already handled above

        Commands::Login => {
            info!("Logging in...");
            client.login(&config.username, &config.password).await?;
            client.sync_once().await?;
            info!("Login successful");
        }

        Commands::Logout => {
            info!("Logging out...");
            client.logout().await?;
            info!("Logout successful");
        }

        Commands::Room(room_cmd) => {
            debug!(?room_cmd, "Executing room command");
            room::handle_room_command(&client, room_cmd).await?;
        }

        Commands::Message(msg_cmd) => {
            debug!(?msg_cmd, "Executing message command");
            message::handle_message_command(&client, msg_cmd).await?;
        }

        Commands::Media(media_cmd) => {
            debug!(?media_cmd, "Executing media command");
            media::handle_media_command(&client, media_cmd).await?;
        }

        Commands::Sync(sync_cmd) => {
            debug!(?sync_cmd, "Executing sync command");
            sync::handle_sync_command(&client, sync_cmd).await?;
        }
    }

    Ok(())
}
