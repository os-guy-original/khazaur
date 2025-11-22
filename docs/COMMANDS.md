---
layout: default
title: Commands Reference
---

# Khazaur Commands Reference

Complete command reference for khazaur, the modern AUR helper.

## Installation Operations

### Install Packages

```bash
# Install a single package
khazaur -S package-name

# Install multiple packages
khazaur -S package1 package2 package3

# Install from specific source using prefix (pacman-style)
khazaur -S aur/yay
khazaur -S core/linux extra/firefox
khazaur -S flatpak/org.mozilla.firefox
khazaur -S snap/discord
khazaur -S debian/htop

# Install from specific source using flags
khazaur -S yay --aur
khazaur -S firefox --flatpak

# Alternative syntax
khazaur install package-name
```

**Source Prefix Syntax:**

Khazaur supports pacman-style source prefixes to explicitly specify where to install from:

- `aur/package` - Install from AUR
- `repo/package` - Install from official repositories (any repo name: core, extra, multilib, community)
- `flatpak/app-id` - Install from Flatpak
- `snap/package` - Install from Snap Store
- `debian/package` - Install from Debian repositories

This is useful when:
- A package exists in multiple sources
- You want to skip the interactive source selection
- You're scripting installations

### Sync Database

```bash
# Update package databases
khazaur -Sy
```

### System Upgrade

```bash
# Full system upgrade (repos + AUR + Snap + Debian index)
khazaur -Syu

# Alternative syntax
khazaur update
```

The system upgrade process:
1. Synchronizes package databases
2. Checks for updates in both official repositories and AUR
3. Shows all available updates in a unified list
4. Upgrades repository packages first
5. Rebuilds and installs updated AUR packages
6. Refreshes Snap packages (if snapd is installed)
7. Updates Debian package index and debtap database (if debtap is installed)

**Unified Upgrade Features:**
- Single confirmation for all updates (repo + AUR)
- Unified display showing all available updates together
- Repository packages marked by repo name, AUR packages marked with [AUR]
- Batch querying of AUR for efficiency
- Optional PKGBUILD review before rebuilding (respects `review_pkgbuild` config)
- Detailed upgrade information showing old → new versions

## Search Operations

### Search for Packages

```bash
# Search all sources (repos + AUR)
khazaur -Ss search-term

# Search AUR only
khazaur -Ss search-term --aur

# Search repositories only
khazaur -Ss search-term --repo

# Alternative syntax
khazaur search search-term
```

### Package Information

```bash
# Show detailed package information
khazaur -Si package-name
```

## Removal Operations

### Remove Packages

```bash
# Remove a package
khazaur -R package-name

# Remove package with dependencies
khazaur -Rs package-name

# Remove package cascading (with dependencies that depend on it)
khazaur -Rc package-name
```

## Query Operations

### Query Installed Packages

```bash
# List all installed packages
khazaur -Q

# Show information about installed package
khazaur -Qi package-name

# List explicitly installed packages
khazaur -Qe

# List orphaned packages
khazaur -Qdt
```

## Local Package Operations

### Install from File

```bash
# Install a local package file
khazaur -U /path/to/package.pkg.tar.zst
```

## Options

### Global Options

- `--noconfirm` - Skip confirmation prompts
- `--aur` - Operate on AUR packages only
- `--repo` - Operate on repository packages only
- `-v, --verbose` - Show debug information and detailed logs

### Shell Completions

Generate shell completion scripts for your shell:

```bash
# Bash
khazaur --completions bash > /usr/share/bash-completion/completions/khazaur

# Zsh
khazaur --completions zsh > /usr/share/zsh/site-functions/_khazaur

# Fish
khazaur --completions fish > ~/.config/fish/completions/khazaur.fish

# PowerShell
khazaur --completions powershell > khazaur.ps1

# Elvish
khazaur --completions elvish > khazaur.elv
```

After generating completions, restart your shell or source the completion file.

### Help

```bash
# Show help
khazaur --help

# Show version
khazaur --version
```

## Verbose Mode

Enable verbose output to see detailed debug information:

```bash
# Install with verbose output
khazaur -S package-name -v

# Search with verbose logging
khazaur -Ss query -v
```

Verbose mode shows:
- HTTP request details
- Download attempts and retries
- Dependency resolution steps
- Build process details

## Unified System Upgrades

Khazaur provides a unified upgrade experience for both repository and AUR packages.

### How It Works

When you run `khazaur -Syu`, the upgrade process:

1. **Syncs databases**: Updates package database information
2. **Checks repo updates**: Uses `pacman -Qu` to find available repository updates
3. **Checks AUR updates**: Batch queries the AUR API for current versions of all installed AUR packages
4. **Compares versions**: Uses `vercmp` to determine which AUR packages have updates available
5. **Shows all updates**: Displays a unified list of all available updates (repo + AUR)
6. **Confirms upgrade**: Asks for single confirmation for all updates (unless `--noconfirm` is used)
7. **Upgrades repos first**: Installs all repository package updates via pacman
8. **Downloads PKGBUILDs**: Downloads the latest PKGBUILD for each AUR package to be upgraded
9. **Reviews PKGBUILDs**: Optionally allows you to review PKGBUILDs before building (if `review_pkgbuild` is enabled)
10. **Rebuilds AUR packages**: Builds and installs each updated AUR package in sequence

### Example Output

```bash
$ khazaur -Syu

:: System Upgrade
:: Synchronizing package databases...
[sync output...]

:: Checking for updates...

:: Packages (5):
  firefox 120.0-1 -> 121.0-1
  linux 6.6.1-1 -> 6.6.2-1
  systemd 255.1-1 -> 255.2-1
  yay 12.0.5-1 -> 12.1.0-1 [AUR]
  paru 2.0.0-1 -> 2.0.1-1 [AUR]

:: Repository: 3, AUR: 2
Proceed with upgrade? [Y/n]: y

:: Upgrading repository packages...
[pacman output...]

:: Upgrading AUR packages...

:: Downloading PKGBUILDs...
✓ yay
✓ paru

:: Building and installing AUR packages...
:: Building yay...
[build output...]
✓ yay upgraded successfully

:: Building paru...
[build output...]
✓ paru upgraded successfully

:: Successfully upgraded 2 AUR package(s)

:: System upgrade complete
```

### Configuration

Control AUR upgrade behavior in `~/.config/khazaur/config.toml`:

```toml
# Review PKGBUILDs before building during upgrades
review_pkgbuild = false

# Skip all confirmations (use with caution)
confirm = true
```

### Skip Confirmations

```bash
# Upgrade without any prompts
khazaur -Syu --noconfirm
```

**Note**: Using `--noconfirm` will skip PKGBUILD review. Only use this if you trust the packages being upgraded.

## Examples

### Common Workflows

#### Install an AUR package

```bash
# Search for a package
khazaur -Ss yay

# View package details
khazaur -Si yay

# Install the package
khazaur -S yay
```

#### Update system

```bash
# Sync databases and upgrade
khazaur -Syu
```

#### Remove unneeded packages

```bash
# Find orphaned packages
khazaur -Qdt

# Remove orphaned packages
khazaur -Rs $(khazaur -Qdtq)
```

#### Install multiple related packages

```bash
khazaur -S package1 package2 package3
```

## PKGBUILD Review

By default, khazaur will show you the PKGBUILD before building AUR packages. This is a security feature to ensure you know what code will be executed on your system.

To skip PKGBUILD review:

```bash
khazaur -S package-name --noconfirm
```

## Cache Directory

Khazaur uses `~/.cache/khazaur/` for caching:

- `~/.cache/khazaur/clone/` - Downloaded PKGBUILD files
- `~/.cache/khazaur/pkg/` - Built package files

To clean the cache:

```bash
rm -rf ~/.cache/khazaur
```

## Viewing PKGBUILDs

### Automatic Display

When installing AUR packages, khazaur shows the PKGBUILD location:

```bash
$ khazaur -S package-name

→ PKGBUILD downloaded to: ~/.cache/khazaur/clone/package-name
→ To view: cat ~/.cache/khazaur/clone/package-name/PKGBUILD
→ To edit: $EDITOR ~/.cache/khazaur/clone/package-name/PKGBUILD
```

### Manual Review

You can review the PKGBUILD before building:

```bash
# In another terminal
cat ~/.cache/khazaur/clone/package-name/PKGBUILD

# Or edit it
vim ~/.cache/khazaur/clone/package-name/PKGBUILD
```

### Interactive Review

PKGBUILDs are pre-downloaded (they're small files) so you can view them instantly.

When installing an AUR package, khazaur prompts:

```
:: PKGBUILD Review
   Press [V]iew, [E]dit, or [S]kip:
```

**To view the PKGBUILD:**
- Press `v` - PKGBUILD displays immediately (already downloaded)
- After viewing, you'll be asked: `Continue with build? [Y/n]:`

**To edit (coming soon):**
- Press `e` - Opens PKGBUILD in `$EDITOR`

**To skip:**
- Press `s`, Enter, or any other key - Proceeds directly to build

### Skip All Reviews

Use `--noconfirm` to skip all prompts:

```bash
khazaur -S package-name --noconfirm
```

## Troubleshooting

### Package not found

If khazaur reports a package is not found:

1. Ensure package databases are up to date: `khazaur -Sy`
2. Check the package name spelling
3. Search for the package: `khazaur -Ss package-name`

### Build failed

If a package build fails:

1. Check the error message for missing dependencies
2. Review the PKGBUILD for any issues
3. Check the AUR page for comments about build issues

### Permission denied

Khazaur requires sudo for installing packages. Ensure you have sudo privileges.
