use structopt::StructOpt;
use url::Url;
use matrix_sdk::{
    Client, SyncSettings, Result,
    ruma::{UserId, events::{SyncMessageEvent, room::message::MessageEventContent}},
};

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
    username: String,

    /// Your matrix password
    #[structopt(short, long, env = "MATRIX_CLI_PASSWORD")]
    password: String,

    /// Generate an API token from your matrix account¬
    /// automatic registration coming later
    //#[structopt(short, long, env = "MATRIX_CLI_TOKEN")]
    //token: String,

    #[structopt(subcommand)]
    subcommands: Option<MatrixCli>,
}

#[derive(StructOpt, Debug)]
enum MatrixCli {
    User {
        #[structopt(subcommand)]
        commands: Option<User>,
    },

    Room {
        #[structopt(subcommand)]
        commands: Option<Room>,
    },
}


#[derive(StructOpt, Debug)]
enum User {
    /// Set the users display name
    SetDisplayName {
        #[structopt(name = "NAME")]
        name: String
    },
}

#[derive(StructOpt, Debug)]
enum Room {
    /// Join a matrix Room
    Join { },
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::from_args();
    let homeserver_url_str = args.homeserver_url;
    let homeserver_url = Url::parse(&homeserver_url_str).unwrap();
    let username = args.username;
    let password = args.password;
    let client = Client::new(homeserver_url).unwrap();
    let response = client
        .login(&username, &password, None, Some("matrix-cli")).await
        .unwrap();

    println!("Logged in as {}, got device_id {} and access_token {}",
             response.user_id, response.device_id, response.access_token);
    if let Some(scmd) = args.subcommands {
        match scmd {
            MatrixCli::User { commands } => {
                if let Some(cmd) = commands {
                    match cmd {
                        User::SetDisplayName { name } => {
                            client.set_display_name(Some(&name)).await.expect("Failed setting display name");
                        },
                    }
                }
            },
            MatrixCli::Room { commands } => {
                println!("{:?}", commands);
            },
        }
    }

    Ok(())
}
