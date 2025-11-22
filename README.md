# Khazaur

<p align="center">
  <img src="res/logo/khazaur.svg" alt="Khazaur Logo" width="200">
</p>

<p align="center">
  <strong>One Package Manager to Rule Them All</strong>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-1.70%2B-orange.svg" alt="Rust"></a>
  <a href="LICENSE"><img src="https://img.shields.io/badge/license-GPL--3.0-blue.svg" alt="License"></a>
</p>

---

## What is Khazaur?

Khazaur is a package manager for Arch Linux that unifies multiple package sources into a single interface. Instead of switching between `pacman`, `yay`, `flatpak`, and `snap`, you can use one tool for everything.

Written in Rust for performance and reliability.

## Features

**Multi-source support**
- Official Arch repositories (via pacman)
- AUR packages (automatic building)
- Flatpak applications from Flathub
- Snap packages from Snap Store
- Debian .deb files (via debtap conversion)

**Unified search**
- Search all sources with one command
- Interactive selection when multiple matches exist
- Filter by source with `--aur`, `--repo`, `--flatpak`, `--snap`, or `--debian`

**Smart behavior**
- Automatic dependency resolution
- 24-hour cache for Debian packages
- Checksum verification
- Conflict detection for package removal
- Optional dependency prompts (flatpak, snapd, debtap)

**Security**
- PKGBUILD review before building
- MD5 verification for Debian packages
- Supports pkexec, sudo, or doas

## Installation

### Using PKGBUILD (Recommended)

```bash
git clone https://github.com/os-guy-original/khazaur.git
cd khazaur
makepkg -si
```

### Manual Build

Requirements:
- Arch Linux or Arch-based distribution
- Rust 1.70+
- pacman and makepkg
- libgit2, libssh2, openssl, zlib (build dependencies)
- Privilege escalation tool (pkexec, sudo, or doas)

```bash
git clone https://github.com/os-guy-original/khazaur.git
cd khazaur
export LIBGIT2_SYS_USE_PKG_CONFIG=1
export LIBSSH2_SYS_USE_PKG_CONFIG=1
cargo build --release
sudo install -Dm755 target/release/khazaur /usr/bin/khazaur
```

### Optional Dependencies

Khazaur will prompt to install these when needed:
- `flatpak` - for Flatpak support
- `snapd` - for Snap support
- `debtap` - for Debian package conversion

## Usage

### Basic commands

```bash
# Search for packages
khazaur -Ss firefox

# Install a package
khazaur -S firefox

# Update package databases
khazaur -Sy

# Upgrade system
khazaur -Syu

# Remove a package
khazaur -R firefox

# Show package info
khazaur -Si firefox
```

### Source-specific operations

```bash
# Search specific sources
khazaur -Ss yay --aur
khazaur -Ss spotify --flatpak
khazaur -Ss discord --snap
khazaur -Ss htop --debian
khazaur -Ss firefox --repo

# Update specific sources
khazaur -Sy --repo          # Update pacman databases only
khazaur -Sy --snap          # Refresh snap packages only
khazaur -Sy --debian        # Update Debian index and debtap
```

### Installing .deb files

```bash
khazaur package.deb
# or
khazaur -S package.deb
```

### Handling dependency conflicts

When removing packages with dependencies, khazaur detects conflicts and prompts for confirmation:

```bash
khazaur -R flatpak
# Shows dependent packages
# Asks if you want to force removal with -Rdd
```

### Skip confirmations

```bash
khazaur -S package-name --noconfirm
```

## Configuration

Config file: `~/.config/khazaur/config.toml`

```toml
use_color = true
confirm = true
review_pkgbuild = false
concurrent_downloads = 4
use_git_clone = true
max_concurrent_requests = 10
request_delay_ms = 100

[rejected_dependencies]
flatpak = false
snapd = false
debtap = false
```

Cache directory: `~/.cache/khazaur/`

## Comparison with other AUR helpers

| Feature | Khazaur | yay | paru |
|---------|---------|-----|------|
| Language | Rust | Go | Rust |
| AUR + repos | ✓ | ✓ | ✓ |
| Flatpak | ✓ | ✗ | ✗ |
| Snap | ✓ | ✗ | ✗ |
| Debian packages | ✓ | ✗ | ✗ |
| Multi-source search | ✓ | ✗ | ✗ |
| Dependency resolution | ✓ | ✓ | ✓ |

Khazaur extends the traditional AUR helper model by adding support for additional package sources. If you only need AUR and repository support, yay and paru are excellent choices. Khazaur is useful when you want to manage Flatpak, Snap, or Debian packages alongside your Arch packages.

## Status

Working:
- [x] AUR package installation
- [x] Repository package installation
- [x] Flatpak support
- [x] Snap support
- [x] Debian package support
- [x] Multi-source search
- [x] Dependency resolution
- [x] Package removal
- [x] Conflict handling
- [x] Configuration file

In progress:
- [ ] AUR package upgrades
- [ ] Shell completions
- [ ] Parallel AUR builds

## Contributing

Contributions are welcome. Please submit pull requests or open issues on GitHub.

## License

Licensed under the GNU General Public License v3.0. See [LICENSE](LICENSE) for details.

## Credits

Inspired by yay and paru. Built with Rust. Thanks to the Arch Linux community.

---

Made by [os-guy-original](https://github.com/os-guy-original)
