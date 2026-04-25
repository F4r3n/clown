## Clown

An IRC client written in Rust, with the goal to be memory and CPU light.

## Features

* Run in terminal
* Multiple channels and servers
* Message logs
* Tab completion
* Link preview on hover
* Scroll with mouse or keyboard
* Auto reconnect
* Spell checking (optional)

## Build

```bash
git clone https://github.com/F4r3n/clown.git
cd clown
cargo build --profile dist
```

### Features

| Feature | Default | Description |
|---------|---------|-------------|
| `website-preview` | on | Link preview when hovering a message |
| `spell-checker` | off | Spell checking via `/spell <language>` |

Build without link preview:

```bash
cargo build --profile dist --no-default-features
```

Build with spell checker:

```bash
cargo build --profile dist --features spell-checker
```

## Commands

All commands are typed in the input bar and start with `/`.

| Command | Description |
|---------|-------------|
| `/connect` | Connect to the server (no-op if already connected) |
| `/quit [reason]` | Disconnect and exit the app |
| `/nick <nickname>` | Change your nickname |
| `/join <channel>` | Join a channel |
| `/part [channel] [reason]` | Leave the current or specified channel |
| `/msg <target> <message>` | Send a private message to a user or channel |
| `/me <action>` | Send an action message |
| `/topic <text>` | Set the topic of the current channel |
| `/close [buffer]` | Close the current buffer, or a named one |
| `/spell [language]` | Load the spellchecker for a language (`fr`, `en`, …), depends on a build feature |
| `/config get <path>` | Read a config value |
| `/config set <path> <value>` | Write a config value |
| `/help` | List all available commands |

## Config

### File location

| Platform | Path |
|----------|------|
| Linux | `$XDG_CONFIG_HOME/clown/clown.toml` (usually `~/.config/clown/clown.toml`) |
| macOS | `~/Library/Application Support/com.share.clown/clown.toml` |
| Windows | `%APPDATA%\share\clown\config\clown.toml` |

### Minimal example

```toml
[[servers]]
name = "Libera"

[servers.connection]
address = "irc.libera.chat"
port = 6697

[servers.login]
nickname = "mynick"

[servers.channels]
list = ["#rust"]
auto_join = true
```

### Config dictionnary

**`[[servers]]`** — one block per server, at least one required

| Key | Required | Default | Description |
|-----|----------|---------|-------------|
| `name` | yes | — | Display name |
| `connection.address` | yes | — | Server hostname |
| `connection.port` | no | `6697` | Port (`6697` = TLS, `6667` = plain) |
| `login.nickname` | yes | — | Your nickname |
| `login.password` | no | — | Server password (sent as `PASS`) |
| `login.real_name` | no | nickname | Real name |
| `login.username` | no | nickname | Username |
| `channels.list` | no | `[]` | Channels to join |
| `channels.auto_join` | no | `false` | Join `channels.list` automatically on connect |

**`[completion]`**

| Key | Default | Description |
|-----|---------|-------------|
| `on_empty_input.suffix` | `""` | Suffix appended after completing a nickname on an empty input |
| `in_message.suffix` | `""` | Suffix appended after completing a nickname mid-message |

**`[nickname_colors]`**

| Key | Default | Description |
|-----|---------|-------------|
| `seed` | `0` | Seed for the automatic colour generator |
| `overrides.<nick>` | — | Hex colour override for a specific nickname, e.g. `f4r3n = "#FFFFFF"` |

**Display**

| Key | Default | Description |
|-----|---------|-------------|
| `discuss.left_bar.time` | `true` | Show timestamp column |
| `discuss.left_bar.nickname` | `true` | Show nickname column |
| `users.enabled` | `true` | Show user list panel |
| `topic.enabled` | `true` | Show topic bar |

![clown](images/clown.png)
![clown_preview](images/clown_preview.png)
