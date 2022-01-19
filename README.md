# matrix-cli
CLI for Matrix written in rust

## Installing

Check the [releases page](https://github.com/iamruinous/matrix-cli/releases/latest) for pre-compiled binaries for many platforms.

### MacOS (via brew)

```sh
brew install iamruinous/matrix-tools/matrix-cli
```

### From Source

```sh
git clone https://github.com/iamruinous/matrix-cli.git
cargo install --path .
matrix-cli --help
```

## Usage

`matrix-cli` supports a number of global options, you can specify them either on the command line, or as environment variables.  You can mix and match however you'd like.

| Env Var | CLI option | Description |
|---------|------------|-------------|
| MATRIX_CLI_HOMESERVER_URL | --homeserver-url | Your matrix server e.g. https://example.com |
| MATRIX_CLI_USERNAME | --username | Your matrix username e.g. user:example.com |
| MATRIX_CLI_PASSWORD | --password | Your matrix password |
| MATRIX_CLI_STORE_PATH | --store-path | Where to store the synchonized state information |
| MATRIX_CLI_SESSION_FILE | --session-file | Where to store or read the saved access token from |

### Example 

```sh
env MATRIX_CLI_HOMESERVER_URL="https://example.com" MATRIX_CLI_USERNAME="user:example.com" MATRIX_CLI_PASSWORD="secret" matrix-cli --session-file "/some/place/session.json" rooms joined-rooms
```

### Subcommands

`matrix-cli` uses subcommands to group different types of commands together. For detailed help and options, run `matrix-cli --help` or for specific subcommand help, run `matrix-cli <subcommand> --help`.

There are two ways of authenticating, username and password, or using a saved access token.

### Password Authentication
```sh
matrix-cli --username="user:example.com" --password="secret" user get-avatar-url
```

### Token Authentication
```sh
matrix-cli --session-file "/some/place/session.json" user get-avatar-url
```

⚠️ In order to use token authentication, you need to login using password authentication first, and pass the `--session-file` option and point it to where you would like the file to be saved. Afer a successful login, the token will be written to the file in JSON format. Please be aware, the token is in plain text, so keep it secret, keep it safe. 🧙

#### Generate Access Token

```sh
matrix-cli --username="user:example.com" --password="secret" --session-file="/some/place/session.json" user get-avatar-url
```

### Optimization

To speed up the synchonization process that happens on every login, you can keep a state store. Ruma uses this to keep track of previous states, encryption information, and more to make the login process much faster for busier accounts. 


```sh
matrix-cli --store-path /some/place
```
