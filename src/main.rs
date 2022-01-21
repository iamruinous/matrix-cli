// TODO: Everything in here needs better error handling

use anyhow::Result;
use chrono::{TimeZone, Utc};
use clap::{Parser, Subcommand};
use std::fs::File;
use std::path::PathBuf;
use tabled::{Style, Table, Tabled};
use tokio::signal;
use url::Url;

use matrix_sdk::{
    config::{ClientConfig, SyncSettings},
    room::Room,
    ruma::api::client::r0::room::create_room::Request as CreateRoomRequest,
    ruma::events::{
        room::message::{
            MessageType, RoomMessageEventContent, SyncRoomMessageEvent, TextMessageEventContent,
        },
        AnyMessageEventContent,
    },
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
    #[clap(name = "message")]
    MessageCmd {
        #[clap(subcommand)]
        commands: Option<MessageCmd>,
    },
    /// Get or set user settings
    #[clap(name = "user")]
    UserCmd {
        #[clap(subcommand)]
        commands: Option<UserCmd>,
    },
    /// Manage rooms
    #[clap(name = "room")]
    RoomCmd {
        #[clap(subcommand)]
        commands: Option<RoomCmd>,
    },
}

#[derive(Subcommand, Debug)]
enum MessageCmd {
    /// Listen for messages in a room
    Listen {
        /// Room name or ID
        #[clap(name = "ROOM")]
        room: String,
    },
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
enum UserCmd {
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
enum RoomCmd {
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

    let client = login(
        args.homeserver_url,
        args.username,
        args.password,
        args.session_file,
        args.store_path,
    )
    .await?;

    // since we called `sync_once` before we entered our sync loop we must pass
    // that sync token to `sync_with_callback`
    // let settings = SyncSettings::default().token(client.sync_token().await.unwrap());

    // let (tx, rx) = unbounded_channel();

    // std::thread::spawn(move || {
    //     loop {
    //         let details = redis.get_message(...).unwrap();
    //         tx.send(details).unwrap();
    //     }
    // });

    // This loop will run every time the redis pubsub service gets a message.
    // while let Some(details) = rx.recv().await {
    //     // handle details
    // }
    tokio::select! {
        _ = sync(&client) => {},
        _ = process_cmd(args.subcommands, &client, hostname) => {
        },

    }
    Ok(())
}

async fn login(
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

    Ok(client)
}

async fn sync(client: &Client) -> Result<(), matrix_sdk::Error> {
    client.sync(SyncSettings::default()).await;

    Ok(())
}

async fn process_cmd(
    subcommands: Option<MatrixCli>,
    client: &Client,
    hostname: &str,
) -> Result<(), anyhow::Error> {
    if let Some(scmd) = subcommands {
        match scmd {
            MatrixCli::MessageCmd { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        MessageCmd::Send { room, msg } => {
                            let room_id = <&RoomId>::try_from(&room[..]).expect("Invalid Room ID");
                            let mroom = client
                                .get_joined_room(room_id)
                                .expect("User has not joined this room");

                            let content = AnyMessageEventContent::RoomMessage(
                                RoomMessageEventContent::text_plain(msg),
                            );

                            mroom.send(content, None).await?;
                        }
                        MessageCmd::Listen { room } => {
                            client
                                .register_event_handler(
                                    |event: SyncRoomMessageEvent, room: Room| async move {
                                        if let Room::Joined(_room) = room {
                                            let sender = event.sender.clone();
                                            let msg_body = match event.content.msgtype {
                                                MessageType::Text(TextMessageEventContent {
                                                    body,
                                                    ..
                                                }) => body,
                                                _ => return,
                                            };
                                            let ts: i64 = event.origin_server_ts.get().into();
                                            let date = Utc.timestamp_millis(ts);
                                            println!(
                                                "From: {}\nDate: {}\nMessage: {}\n",
                                                sender, date, msg_body
                                            );
                                        }
                                    },
                                )
                                .await;

                            println!("Listening to room {}, Ctrl-C to stop", room);
                            signal::ctrl_c().await.expect("Failed to listen for Ctrl-C");
                            println!("Exiting.");
                        }
                    };
                };
            }
            MatrixCli::UserCmd { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        UserCmd::GetDisplayName {} => {
                            match client.display_name().await? {
                                None => println!("Display Name Not Set"),
                                Some(display_name) => {
                                    println!("{}", display_name);
                                }
                            };
                        }
                        UserCmd::SetDisplayName { name } => {
                            client.set_display_name(Some(&name)).await?;
                        }
                        UserCmd::GetAvatarUrl {} => {
                            let avatar_url = client.avatar_url().await?.unwrap();
                            println!("{}", avatar_url);
                        }
                        UserCmd::SetAvatar { file } => {
                            let guess = mime_guess::from_path(file.as_path());
                            let mut image = File::open(file)?;
                            let response =
                                client.upload(&guess.first().unwrap(), &mut image).await?;
                            client.set_avatar_url(Some(&response.content_uri)).await?;
                        }
                        UserCmd::InvitedRooms {} => {
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
                        UserCmd::LeftRooms {} => {
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
                        UserCmd::JoinedRooms {} => {
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
            MatrixCli::RoomCmd { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        RoomCmd::Create {} => {
                            let request = CreateRoomRequest::new();
                            // let room_name = <&RoomName>::try_from(&n[..]).unwrap();
                            let response = client.create_room(request).await?;
                            println!("{:?}", response);
                        }
                        RoomCmd::Join { room } => {
                            let room_id = <&RoomOrAliasId>::try_from(&room[..]).unwrap();
                            let server_name: Box<ServerName> = <&ServerName>::try_from(hostname)
                                .unwrap()
                                .try_into()
                                .unwrap();
                            client
                                .join_room_by_id_or_alias(room_id, &[server_name])
                                .await?;
                        }
                        RoomCmd::Leave { room } => {
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
    };

    Ok(())
}
