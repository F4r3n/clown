## Clown

An IRC client written in Rust.

## Features

* run in Terminal
* multiple channel
* preview when hovering message
* scroll with mouse/keyboard
* auto reconnect

## Build

```bash
git clone https://github.com/F4r3n/clown.git
cd clown
cargo build --release
```

## Config

In Roaming/share/clown/config/clown.toml

```toml
[connection_config]
address = "localhost"
port = 6697

[login_config]
nickname = "nickname"
real_name = "real"
username = "username"
password = "password"
channel = "#rust-spam"

[client_config]
auto_join = true
```

![clown](images/clown.png)
![clown_preview](images/clown_preview.png)
