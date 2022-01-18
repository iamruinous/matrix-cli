use anyhow::Result;
use matrix_sdk::Client;
use std::fs::File;
use std::path::PathBuf;
use structopt::StructOpt;
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
    /// Upload the provided image and set it as the users avatar
    SetAvatar {
        #[structopt(name = "FILE")]
        file: PathBuf,
    },
}

// #[derive(StructOpt, Debug)]
// enum Room {
//     /// Join a matrix Room
//     Join {},
// }

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    let homeserver_url_str = args.homeserver_url;
    let homeserver_url = Url::parse(&homeserver_url_str).unwrap();

    let session_file = args.session_file;
    let session_file_exists = match &session_file {
        None => false,
        Some(f) => f.exists(),
    };

    let client = Client::new(homeserver_url).unwrap();
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
            let session: matrix_sdk::Session =
                serde_json::from_reader(session_path)?;
            client.restore_login(session).await?;
        }
    };

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
                        User::SetAvatar { file } => {
                            let guess = mime_guess::from_path(file.as_path());
                            let mut image = File::open(file)?;
                            let response =
                                client.upload(&guess.first().unwrap(), &mut image).await?;
                            client.set_avatar_url(Some(&response.content_uri)).await?;
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
