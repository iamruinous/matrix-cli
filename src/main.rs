use structopt::StructOpt;

#[derive(StructOpt, Debug)]
/// matrix-cli
///
/// Use matrix-cli for simple matrix commands
#[structopt(name = "matrix-cli", about = "A cli for matrix", author)]
struct Cli {
    /// This is your matrix homeserver: e.g. matrix.org 
    #[structopt(short, long, env = "MATRIX_CLI_HOMESERVER")]
    homeserver: String,

    /// Generate an API token from your matrix account¬
    /// automatic registration coming later
    #[structopt(short, long, env = "MATRIX_CLI_TOKEN")]
    token: String,

    #[structopt(subcommand)]
    subcommands: Option<MatrixCli>,
}

#[derive(StructOpt, Debug)]
enum MatrixCli {

    Room {
        #[structopt(subcommand)]
        commands: Option<Room>,
    },
}

#[derive(StructOpt, Debug)]
enum Room {
    /// Join a matrix Room
    Join { },
}

fn main() {
    let args = Cli::from_args();
    let homeserver = args.homeserver;
    let token = args.token;
    if let Some(scmd) = args.subcommands {
        match scmd {
            MatrixCli::Room { commands } => {
                println!("{:?}", commands);
                println!("{} {}", homeserver, token);

                return;
            },
        }
    }
}
