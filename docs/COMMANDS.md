# Khazaur Commands Reference

Complete command reference for khazaur, the modern AUR helper.

## Installation Operations

### Install Packages

```bash
# Install a single package
khazaur -S package-name

# Install multiple packages
khazaur -S package1 package2 package3

# Alternative syntax
khazaur install package-name
```

### Sync Database

```bash
# Update package databases
khazaur -Sy
```

### System Upgrade

```bash
# Full system upgrade (repos + AUR)
khazaur -Syu

# Alternative syntax
khazaur update
```

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
