---
layout: default
title: Configuration
---

# Configuration

Khazaur can be customized through a configuration file located at `~/.config/khazaur/config.toml`.

## Configuration File

The configuration file is automatically created with default values on first run. You can edit it to customize khazaur's behavior.

### Location

```
~/.config/khazaur/config.toml
```

### Default Configuration

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

## Configuration Options

### General Settings

#### `use_color`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Enable or disable colored output in the terminal.

```toml
use_color = true
```

#### `confirm`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Ask for confirmation before performing operations. Set to `false` to skip confirmations (same as using `--noconfirm`).

```toml
confirm = true
```

#### `review_pkgbuild`
- **Type**: Boolean
- **Default**: `false`
- **Description**: Automatically prompt to review PKGBUILDs before building AUR packages.

```toml
review_pkgbuild = true
```

### Download Settings

#### `concurrent_downloads`
- **Type**: Integer
- **Default**: `4`
- **Description**: Number of packages to download concurrently.

```toml
concurrent_downloads = 4
```

#### `use_git_clone`
- **Type**: Boolean
- **Default**: `true`
- **Description**: Use git clone instead of tarball download for AUR packages (generally faster).

```toml
use_git_clone = true
```

### Rate Limiting

#### `max_concurrent_requests`
- **Type**: Integer
- **Default**: `10`
- **Description**: Maximum number of concurrent HTTP requests to the AUR API.

```toml
max_concurrent_requests = 10
```

#### `request_delay_ms`
- **Type**: Integer
- **Default**: `100`
- **Description**: Delay in milliseconds between AUR API requests to avoid rate limiting.

```toml
request_delay_ms = 100
```

### Optional Dependencies

The `[rejected_dependencies]` section tracks which optional dependencies you've chosen not to install.

#### `flatpak`
- **Type**: Boolean
- **Default**: `false`
- **Description**: Set to `true` if you've chosen "Never ask again" for flatpak installation.

#### `snapd`
- **Type**: Boolean
- **Default**: `false`
- **Description**: Set to `true` if you've chosen "Never ask again" for snapd installation.

#### `debtap`
- **Type**: Boolean
- **Default**: `false`
- **Description**: Set to `true` if you've chosen "Never ask again" for debtap installation.

```toml
[rejected_dependencies]
flatpak = false
snapd = false
debtap = false
```

## Cache Directories

Khazaur stores cache and build files in `~/.cache/khazaur/`:

- `~/.cache/khazaur/clone/` - Downloaded PKGBUILDs and source files
- `~/.cache/khazaur/pkg/` - Built package files
- `~/.cache/khazaur/debian/` - Cached Debian packages (24-hour cache)

### Clearing Cache

To clear the cache:

```bash
rm -rf ~/.cache/khazaur/
```

Khazaur will recreate the directories as needed.

## Example Configurations

### Minimal Confirmations

For users who want fewer prompts:

```toml
use_color = true
confirm = false
review_pkgbuild = false
concurrent_downloads = 8
use_git_clone = true
max_concurrent_requests = 15
request_delay_ms = 50

[rejected_dependencies]
flatpak = false
snapd = false
debtap = false
```

### Security-Focused

For users who want to review everything:

```toml
use_color = true
confirm = true
review_pkgbuild = true
concurrent_downloads = 2
use_git_clone = true
max_concurrent_requests = 5
request_delay_ms = 200

[rejected_dependencies]
flatpak = false
snapd = false
debtap = false
```

### Fast Downloads

For users with good internet connections:

```toml
use_color = true
confirm = true
review_pkgbuild = false
concurrent_downloads = 10
use_git_clone = true
max_concurrent_requests = 20
request_delay_ms = 50

[rejected_dependencies]
flatpak = false
snapd = false
debtap = false
```

## Resetting Configuration

To reset to default configuration, simply delete the config file:

```bash
rm ~/.config/khazaur/config.toml
```

Khazaur will create a new one with default values on next run.
