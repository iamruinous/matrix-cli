// TODO: Everything in here needs better error handling

use anyhow::Result;
use matrix_sdk::config::SyncSettings;
use matrix_sdk::ruma::api::client::r0::room::create_room::Request as CreateRoomRequest;
use matrix_sdk::ruma::identifiers::RoomName;
use matrix_sdk::ruma::{RoomOrAliasId, ServerName, RoomId};
use matrix_sdk::Client;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;
use tabled::{Style, Table, Tabled};
use url::Url;

#[derive(StructOpt, Debug)]
/// matrix-cli
///
/// Use matrix-cli for simple matrix commands
#[structopt(name = "matrix-cli", about = "A cli for matrix", author)]
struct Cli {
    /// This is your matrix homeserver: e.g. https://matrix.org
    #[structopt(short, long, env = "MATRIX_CLI_HOMESERVER_URL")]
    homeserver_url: String,

    /// Your matrix username
    #[structopt(short, long, env = "MATRIX_CLI_USERNAME")]
    username: Option<String>,

    /// Your matrix password
    #[structopt(short, long, env = "MATRIX_CLI_PASSWORD")]
    password: Option<String>,

    /// Use or store the session information here
    #[structopt(short, long, env = "MATRIX_CLI_SESSION_FILE")]
    session_file: Option<PathBuf>,

    #[structopt(subcommand)]
    subcommands: Option<MatrixCli>,
}

#[derive(StructOpt, Debug)]
enum MatrixCli {
    /// Get or set user settings
    User {
        #[structopt(subcommand)]
        commands: Option<User>,
    },
    /// Manage rooms
    Room {
        #[structopt(subcommand)]
        commands: Option<Room>,
    },
}

#[derive(StructOpt, Debug)]
enum User {
    /// Gets the users display name
    GetDisplayName {},
    /// Set the users display name
    SetDisplayName {
        #[structopt(name = "NAME")]
        name: String,
    },
    /// Get the current avatar url
    GetAvatarUrl {},
    /// Upload the provided image and set it as the users avatar
    SetAvatar {
        #[structopt(name = "FILE")]
        file: PathBuf,
    },
    /// List the rooms a user is invited to
    InvitedRooms {},
    /// List the rooms a user is currently in
    JoinedRooms {},
    /// List the rooms a user has left
    LeftRooms {},
}

#[derive(StructOpt, Debug)]
enum Room {
    /// Create a matrix room
    Create {
        /// Room name or ID
        #[structopt(short, long)]
        name: Option<String>,
    },
    /// Join a matrix room
    Join {
        /// Room name or ID
        #[structopt(name = "ROOM")]
        room: String,
    },
    /// Leave a matrix room
    Leave {
        /// Room name or ID
        #[structopt(name = "ROOM")]
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
async fn main() -> Result<()> {
    let args = Cli::from_args();
    let homeserver_url_str = args.homeserver_url;
    let homeserver_url = Url::parse(&homeserver_url_str).expect("Could not parse homeserver_url");
    let hostname = homeserver_url.host_str().unwrap();

    let session_file = args.session_file;
    let session_file_exists = match &session_file {
        None => false,
        Some(f) => f.exists(),
    };

    let client = Client::new(homeserver_url.clone()).unwrap();
    match session_file_exists {
        false => {
            let username = args.username.unwrap();
            let password = args.password.unwrap();
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

    if let Some(scmd) = args.subcommands {
        match scmd {
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
                                let display_name = room.name().unwrap_or("".to_owned());
                                let room_alias = match room.canonical_alias() {
                                    None => "".to_owned(),
                                    Some(alias) => format!("#{}:{}", alias.alias(), room_id.server_name()),
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
                                let display_name = room.name().unwrap_or("".to_owned());
                                let room_alias = match room.canonical_alias() {
                                    None => "".to_owned(),
                                    Some(alias) => format!("#{}:{}", alias.alias(), room_id.server_name()),
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
                                let display_name = room.name().unwrap_or("".to_owned());
                                let room_alias = match room.canonical_alias() {
                                    None => "".to_owned(),
                                    Some(alias) => format!("#{}:{}", alias.alias(), room_id.server_name()),
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
                        Room::Create { name } => {
                            let mut request = CreateRoomRequest::new();
                            let room_name: Option<&RoomName> = match name {
                                None => None,
                                Some(n) => Some(<&RoomName>::try_from(&n[..]).unwrap()),
                            };
                            request.name = room_name;
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
                            let room = client.get_joined_room(room_id).expect("User does not belong to this room");
                            room.leave().await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
