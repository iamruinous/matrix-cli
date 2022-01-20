// TODO: Everything in here needs better error handling

use anyhow::Result;
use std::fs::File;
use std::path::PathBuf;
use tabled::{Style, Table, Tabled};
use url::Url;
use clap::{Parser, Subcommand};

use matrix_sdk::{
    config::{ClientConfig, SyncSettings},
    ruma::api::client::r0::room::create_room::Request as CreateRoomRequest,
    ruma::events::{room::message::RoomMessageEventContent, AnyMessageEventContent},
    ruma::{RoomId, RoomOrAliasId, ServerName},
    Client,
};

/// matrix-cli
///
/// Use matrix-cli for simple matrix commands
#[derive(Parser, Debug)]
#[clap(author, version, about, long_about = None)]
struct Cli {
    /// This is your matrix homeserver: e.g. https://matrix.org
    #[clap(short, long, env = "MATRIX_CLI_HOMESERVER_URL")]
    homeserver_url: String,

    /// Your matrix username
    #[clap(short, long, env = "MATRIX_CLI_USERNAME")]
    username: Option<String>,

    /// Your matrix password
    #[clap(short, long, env = "MATRIX_CLI_PASSWORD")]
    password: Option<String>,

    /// Use or store the session information here
    #[clap(short, long, env = "MATRIX_CLI_SESSION_FILE")]
    session_file: Option<PathBuf>,

    /// Store state information here
    #[clap(long, env = "MATRIX_CLI_STORE_PATH")]
    store_path: Option<PathBuf>,

    #[clap(subcommand)]
    subcommands: Option<MatrixCli>,
}

#[derive(Subcommand, Debug)]
enum MatrixCli {
    /// Send and receive messages
    Message {
        #[clap(subcommand)]
        commands: Option<Message>,
    },
    /// Get or set user settings
    User {
        #[clap(subcommand)]
        commands: Option<User>,
    },
    /// Manage rooms
    Room {
        #[clap(subcommand, name="foom")]
        commands: Option<Room>,
    },
}

#[derive(Subcommand, Debug)]
enum Message {
    /// Send a plain text message to a room
    Send {
        /// Room name or ID
        #[clap(name = "ROOM")]
        room: String,
        /// Message to send (plain text)
        #[clap(name = "MSG")]
        msg: String,
    },
}

#[derive(Subcommand, Debug)]
enum User {
    /// Gets the users display name
    GetDisplayName {},
    /// Set the users display name
    SetDisplayName {
        #[clap(name = "NAME")]
        name: String,
    },
    /// Get the current avatar url
    GetAvatarUrl {},
    /// Upload the provided image and set it as the users avatar
    SetAvatar {
        #[clap(name = "FILE")]
        file: PathBuf,
    },
    /// List the rooms a user is invited to
    InvitedRooms {},
    /// List the rooms a user is currently in
    JoinedRooms {},
    /// List the rooms a user has left
    LeftRooms {},
}

#[derive(Subcommand, Debug)]
enum Room {
    /// Create a matrix room
    Create {},
    /// Join a matrix room
    Join {
        /// Room name or ID
        #[clap(name = "ROOM")]
        room: String,
    },
    /// Leave a matrix room
    Leave {
        /// Room name or ID
        #[clap(name = "ROOM")]
        room: String,
    },
}

#[derive(Tabled)]
struct RoomRow {
    id: String,
    alias: String,
    description: String,
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let args = Cli::parse();
    let homeserver_url_str = args.homeserver_url.clone();
    let homeserver_url = Url::parse(&homeserver_url_str).expect("Could not parse homeserver_url");
    let hostname = homeserver_url.host_str().unwrap();

    let client = login_and_sync(
        args.homeserver_url,
        args.username,
        args.password,
        args.session_file,
        args.store_path,
    )
    .await?;

    if let Some(scmd) = args.subcommands {
        match scmd {
            MatrixCli::Message { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        Message::Send { room, msg } => {
                            let room_id = <&RoomId>::try_from(&room[..]).expect("Invalid Room ID");
                            let mroom = client
                                .get_joined_room(room_id)
                                .expect("User has not joined this room");

                            let content = AnyMessageEventContent::RoomMessage(
                                RoomMessageEventContent::text_plain(msg),
                            );

                            mroom.send(content, None).await?;
                        }
                    };
                };
            }
            MatrixCli::User { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        User::GetDisplayName {} => {
                            match client.display_name().await? {
                                None => println!("Display Name Not Set"),
                                Some(display_name) => {
                                    println!("{}", display_name);
                                }
                            };
                        }
                        User::SetDisplayName { name } => {
                            client.set_display_name(Some(&name)).await?;
                        }
                        User::GetAvatarUrl {} => {
                            let avatar_url = client.avatar_url().await?.unwrap();
                            println!("{}", avatar_url);
                        }
                        User::SetAvatar { file } => {
                            let guess = mime_guess::from_path(file.as_path());
                            let mut image = File::open(file)?;
                            let response =
                                client.upload(&guess.first().unwrap(), &mut image).await?;
                            client.set_avatar_url(Some(&response.content_uri)).await?;
                        }
                        User::InvitedRooms {} => {
                            let mut data: Vec<RoomRow> = Vec::new();
                            for room in client.invited_rooms() {
                                let room_id = room.room_id();
                                let display_name = room.name().unwrap_or_else(|| "".to_owned());
                                let room_alias = match room.canonical_alias() {
                                    None => "".to_owned(),
                                    Some(alias) => {
                                        format!("#{}:{}", alias.alias(), room_id.server_name())
                                    }
                                };
                                let rr = RoomRow {
                                    id: room_id.to_string(),
                                    alias: room_alias,
                                    description: display_name,
                                };
                                data.push(rr);
                            }
                            let t = Table::new(&data).with(Style::GITHUB_MARKDOWN);
                            println!("{}", t);
                        }
                        User::LeftRooms {} => {
                            let mut data: Vec<RoomRow> = Vec::new();
                            for room in client.left_rooms() {
                                let room_id = room.room_id();
                                let display_name = room.name().unwrap_or_else(|| "".to_owned());
                                let room_alias = match room.canonical_alias() {
                                    None => "".to_owned(),
                                    Some(alias) => {
                                        format!("#{}:{}", alias.alias(), room_id.server_name())
                                    }
                                };
                                let rr = RoomRow {
                                    id: room_id.to_string(),
                                    alias: room_alias,
                                    description: display_name,
                                };
                                data.push(rr);
                            }
                            let t = Table::new(&data).with(Style::GITHUB_MARKDOWN);
                            println!("{}", t);
                        }
                        User::JoinedRooms {} => {
                            let mut data: Vec<RoomRow> = Vec::new();
                            for room in client.joined_rooms() {
                                let room_id = room.room_id();
                                let display_name = room.name().unwrap_or_else(|| "".to_owned());
                                let room_alias = match room.canonical_alias() {
                                    None => "".to_owned(),
                                    Some(alias) => {
                                        format!("#{}:{}", alias.alias(), room_id.server_name())
                                    }
                                };
                                let rr = RoomRow {
                                    id: room_id.to_string(),
                                    alias: room_alias,
                                    description: display_name,
                                };
                                data.push(rr);
                            }
                            let t = Table::new(&data).with(Style::PSQL);
                            println!("{}", t);
                        }
                    }
                }
            }
            MatrixCli::Room { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        Room::Create {} => {
                            let request = CreateRoomRequest::new();
                            // let room_name = <&RoomName>::try_from(&n[..]).unwrap();
                            let response = client.create_room(request).await?;
                            println!("{:?}", response);
                        }
                        Room::Join { room } => {
                            let room_id = <&RoomOrAliasId>::try_from(&room[..]).unwrap();
                            let server_name: Box<ServerName> = <&ServerName>::try_from(hostname)
                                .unwrap()
                                .try_into()
                                .unwrap();
                            client
                                .join_room_by_id_or_alias(room_id, &[server_name])
                                .await?;
                        }
                        Room::Leave { room } => {
                            let room_id = <&RoomId>::try_from(&room[..]).expect("Invalid Room ID");
                            let room = client
                                .get_joined_room(room_id)
                                .expect("User does not belong to this room");
                            room.leave().await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

async fn login_and_sync(
    homeserver_url_str: String,
    username: Option<String>,
    password: Option<String>,
    session_file: Option<PathBuf>,
    store_path: Option<PathBuf>,
) -> Result<Client, matrix_sdk::Error> {
    let homeserver_url = Url::parse(&homeserver_url_str).expect("Could not parse homeserver_url");
    let session_file_exists = match &session_file {
        None => false,
        Some(sf) => sf.exists(),
    };

    let mut config = ClientConfig::new();
    if let Some(store_path) = store_path {
        config = config.store_path(store_path);
    };
    let client = Client::new_with_config(homeserver_url.clone(), config)
        .expect("Could not connect to homeserver");
    match session_file_exists {
        false => {
            let username = username.expect("Missing username");
            let password = password.expect("Missing password");
            let _response = client
                .login(&username, &password, None, Some("matrix-cli"))
                .await?;

            // Only write the session if the session_file is specified
            if session_file.is_some() {
                let session_path = File::create(session_file.unwrap())?;
                let session = client.session().await.unwrap();

                serde_json::to_writer(session_path, &session)?;
            }
        }
        true => {
            let session_path = File::open(session_file.unwrap())?;
            let session: matrix_sdk::Session = serde_json::from_reader(session_path)?;
            client.restore_login(session).await?;
        }
    };

    client.sync_once(SyncSettings::default()).await.unwrap();

    Ok(client)
}
