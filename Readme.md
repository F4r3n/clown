# clown

**clown** is a modern IRC client written in Rust, designed to be simple, secure, and easy to install on Windows and Linux.

---

## Overview

- **Quick connection** to IRC servers (with or without TLS/SSL)
- **Easy configuration** via a TOML file
- **Automated installation** with scripts and prebuilt releases
- Compatible with Windows, Linux, and macOS

---

## Installation

### ⚡ Recommended Method (Prebuilt Binary)

#### **Windows**

Open PowerShell and run:
```powershell
powershell -ExecutionPolicy Bypass -c "irm https://github.com/F4r3n/clown/releases/download/v0.1.2/clown-installer.ps1 | iex"
```
This downloads and installs the latest stable version.

#### **Linux**

Open your terminal and run:
```sh
curl --proto '=https' --tlsv1.2 -LsSf https://github.com/F4r3n/clown/releases/download/v0.1.2/clown-installer.sh | sh
```

#### **Other platforms / manual installation**

Go to the [releases page](https://github.com/F4r3n/clown/releases) and download the file matching your platform:
- Windows: `.zip` or `.msi`
- Linux: `.tar.xz`

Unpack it and place the binary in your `$PATH` or run it directly.

---

## Configuration

The client uses a TOML file for configuration.
After the first launch, this file will be created automatically:

- **Windows**:  
  `C:\Users\<YourName>\AppData\Roaming\clown\clown.toml`
- **Linux**:  
  `~/.config/clown/clown.toml`

### Example content for `clown.toml`

```toml
connection_config = { address = "irc.example.com", port = 6697 }
login_config = { nickname = "YourNick", password = "", real_name = "YourName", username = "username", channel = "#channel" }
client_config = { auto_join = true }
```

Edit this file to set your server, nickname, and default IRC channel.

---

## Manual Compilation (optional)

If you want to build from source:

```sh
git clone https://github.com/F4r3n/clown.git
cd clown
cargo build --release
```

The binary will be available in `target/release/clown`.

---

## Usage

Run the client after installation:

- **Windows**:  
  Search "clown" in the start menu or run the binary.
- **Linux**:  
  In the terminal:
  ```sh
  clown
  ```

---

## Project structure

- `clown/` (main client)
- `clown-core/` (IRC logic and connection)
- `clown-parser/` (IRC message parsing)

---

## License

MIT

---

**Need help?**  
Open an issue on GitHub or check the releases page for the latest information.
