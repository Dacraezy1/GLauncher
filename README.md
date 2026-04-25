# GLauncher

A modern, fast, native Minecraft Java Edition launcher for Linux — built with **Rust**, **GTK4**, and **libadwaita**.

[![Build & Release](https://github.com/Dacraezy1/GLauncher/actions/workflows/build.yml/badge.svg)](https://github.com/Dacraezy1/GLauncher/actions/workflows/build.yml)
[![License: GPL v3](https://img.shields.io/badge/License-GPLv3-blue.svg)](https://www.gnu.org/licenses/gpl-3.0)

![GLauncher Screenshot](assets/icons/io.github.Dacraezy1.GLauncher.svg)

## Features

- **Microsoft Account Login** — Full Xbox Live → XSTS → Minecraft authentication chain (PKCE OAuth2)
- **Offline Mode** — Just enter a username and play
- **All Minecraft Versions** — Releases, snapshots, old beta, old alpha
- **Mod Loaders** — Fabric, Forge, Quilt, NeoForge (choose loader version per instance)
- **Modrinth Browser** — Search and install mods directly from Modrinth
- **CurseForge Browser** — Search and install mods from CurseForge (requires free API key)
- **Java Management** — Auto-detect system Java, download Java 8–24 via Eclipse Adoptium
- **JVM Tuning** — Aikar's flags, G1GC, ZGC, Shenandoah GC presets; custom extra args
- **Per-Instance Settings** — Memory, GC preset, custom Java path, window size, notes
- **Native UI** — GTK4 + libadwaita, follows your GNOME theme (light/dark/system)

## Installation

### From GitHub Releases (recommended)

Download the latest release from the [Releases page](https://github.com/Dacraezy1/GLauncher/releases).

**Raw binary (requires GTK4 + libadwaita on your system):**
```sh
# Arch Linux
sudo pacman -S gtk4 libadwaita

chmod +x glauncher-linux-x86_64
./glauncher-linux-x86_64

# Optional: install to PATH
sudo cp glauncher-linux-x86_64 /usr/local/bin/glauncher
```

**AppImage (portable, no system deps needed):**
```sh
chmod +x GLauncher-x86_64.AppImage
./GLauncher-x86_64.AppImage
```

### Building from source

```sh
# Install dependencies (Arch Linux)
sudo pacman -S gtk4 libadwaita rust cargo pkg-config

# Clone and build
git clone https://github.com/Dacraezy1/GLauncher
cd GLauncher
cargo build --release

# Run
./target/release/glauncher
```

## Usage

### First Launch

1. Open GLauncher
2. Go to **Accounts** → Add a Microsoft or Offline account
3. Go to **Instances** → **New Instance** → choose version & mod loader
4. Hit the ▶ play button

### Microsoft Login

GLauncher uses a browser-based OAuth2 flow:
1. Click **Login** → your browser opens the Microsoft login page
2. Sign in with your Microsoft account
3. After login, copy the full URL from your browser's address bar
4. Paste it back in GLauncher

No passwords are stored — only the OAuth refresh token (same as any Minecraft launcher).

### Offline Mode

Create an offline account with any username. You can play singleplayer or on servers with `online-mode=false`.

### CurseForge API Key

CurseForge requires a free API key:
1. Go to https://console.curseforge.com → Create a project → API Keys
2. Paste the key in **Settings → CurseForge API Key**

## Data Location

All data is stored in `~/.local/share/GLauncher/`:
```
~/.local/share/GLauncher/
  instances/    — Per-instance data (minecraft dir, mods, saves...)
  versions/     — Cached Minecraft version JARs and metadata
  assets/       — Game assets (sounds, textures)
  libraries/    — Game libraries / loader JARs
  java/         — Bundled Java installations
  accounts.json — Account data (no plaintext passwords)
  config.json   — Launcher settings
```

## Contributing

PRs are welcome! This is a GPLv3 open-source project.

1. Fork the repository
2. Create a feature branch
3. Open a pull request

## License

GLauncher is licensed under the **GNU General Public License v3.0**.  
See [LICENSE](LICENSE) for details.

---

*GLauncher is not affiliated with Mojang or Microsoft.*
