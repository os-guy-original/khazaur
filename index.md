---
layout: default
---

<div class="hero">
  <div class="hero-content">
    <img src="res/logo/khazaur.svg" alt="Khazaur Logo" class="hero-logo">
    <h1>Khazaur</h1>
    <p class="tagline">One Package Manager to Rule Them All</p>
    <p class="subtitle">Unified package management for Arch Linux — repos, AUR, Flatpak, Snap, and Debian packages in one tool.</p>
    <div class="hero-buttons">
      <a href="#install" class="btn btn-primary">Get Started</a>
      <a href="https://github.com/os-guy-original/khazaur" class="btn btn-secondary">GitHub</a>
    </div>
  </div>
</div>

<section id="features" class="features">
  <h2>Features</h2>
  
  <div class="feature-grid">
    <div class="feature-card">
      <h3>Multi-source support</h3>
      <p>Install from official repos, AUR, Flatpak, Snap, and Debian packages using a single command interface.</p>
    </div>

    <div class="feature-card">
      <h3>Unified search</h3>
      <p>Search across all package sources simultaneously. Results show which source each package comes from.</p>
    </div>

    <div class="feature-card">
      <h3>Intelligent caching</h3>
      <p>Debian packages are cached for 24 hours with MD5 verification to reduce bandwidth usage.</p>
    </div>

    <div class="feature-card">
      <h3>Interactive selection</h3>
      <p>When multiple sources have the same package, choose which one to install through an interactive menu.</p>
    </div>

    <div class="feature-card">
      <h3>Security features</h3>
      <p>Review PKGBUILDs before building, verify checksums, and use pkexec/sudo/doas for privilege escalation.</p>
    </div>

    <div class="feature-card">
      <h3>Clear output</h3>
      <p>Progress indicators, color-coded messages, and organized output make operations easy to follow.</p>
    </div>

    <div class="feature-card">
      <h3>Debian package support</h3>
      <p>Install .deb files directly with automatic conversion using debtap.</p>
    </div>

    <div class="feature-card">
      <h3>Optional dependency handling</h3>
      <p>Prompts to install flatpak, snapd, or debtap when you first try to use them.</p>
    </div>

    <div class="feature-card">
      <h3>Automatic setup</h3>
      <p>Enables systemd services for snapd and creates necessary symlinks automatically.</p>
    </div>

    <div class="feature-card">
      <h3>Dependency resolution</h3>
      <p>Automatically resolves dependencies and determines the correct build order for AUR packages.</p>
    </div>

    <div class="feature-card">
      <h3>Source filtering</h3>
      <p>Use --aur, --repo, --flatpak, --snap, or --debian to target specific package sources.</p>
    </div>

    <div class="feature-card">
      <h3>Built with Rust</h3>
      <p>Memory-safe and fast. Compiled binary with no runtime dependencies.</p>
    </div>
  </div>
</section>

<section id="install" class="install">
  <h2>Installation</h2>
  
  <div class="install-steps">
    <div class="step">
      <h3>Build from source</h3>
      <div class="code-block">
        <code>git clone https://github.com/os-guy-original/khazaur.git
cd khazaur
cargo build --release
sudo cp target/release/khazaur /usr/local/bin/</code>
      </div>
    </div>

    <div class="step">
      <h3>Requirements</h3>
      <ul>
        <li>Arch Linux or Arch-based distribution</li>
        <li>Rust 1.70 or newer</li>
        <li>pacman and makepkg</li>
        <li>pkexec, sudo, or doas</li>
      </ul>
    </div>

    <div class="step">
      <h3>Optional dependencies</h3>
      <p>Khazaur will prompt to install these when needed:</p>
      <ul>
        <li>flatpak (for Flatpak applications)</li>
        <li>snapd (for Snap packages)</li>
        <li>debtap (for Debian package conversion)</li>
      </ul>
    </div>
  </div>
</section>

<section id="usage" class="usage">
  <h2>Usage</h2>

  <div class="usage-examples">
    <div class="example">
      <h3>Search all sources</h3>
      <div class="code-block">
        <code>khazaur -Ss firefox</code>
      </div>
      <p class="example-note">Shows results from all available sources</p>
    </div>

    <div class="example">
      <h3>Install a package</h3>
      <div class="code-block">
        <code>khazaur -S package-name</code>
      </div>
      <p class="example-note">Interactive selection if multiple sources available</p>
    </div>

    <div class="example">
      <h3>Search specific sources</h3>
      <div class="code-block">
        <code>khazaur -Ss yay --aur
khazaur -Ss spotify --flatpak
khazaur -Ss discord --snap</code>
      </div>
      <p class="example-note">Filter by source type</p>
    </div>

    <div class="example">
      <h3>Update databases</h3>
      <div class="code-block">
        <code>khazaur -Sy
khazaur -Sy --repo
khazaur -Sy --snap
khazaur -Sy --debian</code>
      </div>
      <p class="example-note">Update all or specific sources</p>
    </div>

    <div class="example">
      <h3>System upgrade</h3>
      <div class="code-block">
        <code>khazaur -Syu</code>
      </div>
      <p class="example-note">Update databases and upgrade packages</p>
    </div>

    <div class="example">
      <h3>Install .deb file</h3>
      <div class="code-block">
        <code>khazaur package.deb</code>
      </div>
      <p class="example-note">Automatic conversion with debtap</p>
    </div>

    <div class="example">
      <h3>Remove a package</h3>
      <div class="code-block">
        <code>khazaur -R package-name</code>
      </div>
      <p class="example-note">Detects and handles dependency conflicts</p>
    </div>

    <div class="example">
      <h3>Package information</h3>
      <div class="code-block">
        <code>khazaur -Si package-name</code>
      </div>
      <p class="example-note">Shows detailed package information</p>
    </div>
  </div>
</section>

<section id="comparison" class="comparison">
  <h2>Comparison</h2>
  
  <div class="comparison-table">
    <table>
      <thead>
        <tr>
          <th>Feature</th>
          <th>Khazaur</th>
          <th>yay</th>
          <th>paru</th>
        </tr>
      </thead>
      <tbody>
        <tr>
          <td>Language</td>
          <td>Rust</td>
          <td>Go</td>
          <td>Rust</td>
        </tr>
        <tr>
          <td>AUR + repos</td>
          <td>✓</td>
          <td>✓</td>
          <td>✓</td>
        </tr>
        <tr>
          <td>Flatpak</td>
          <td>✓</td>
          <td>✗</td>
          <td>✗</td>
        </tr>
        <tr>
          <td>Snap</td>
          <td>✓</td>
          <td>✗</td>
          <td>✗</td>
        </tr>
        <tr>
          <td>Debian packages</td>
          <td>✓</td>
          <td>✗</td>
          <td>✗</td>
        </tr>
        <tr>
          <td>Multi-source search</td>
          <td>✓</td>
          <td>✗</td>
          <td>✗</td>
        </tr>
        <tr>
          <td>Dependency resolution</td>
          <td>✓</td>
          <td>✓</td>
          <td>✓</td>
        </tr>
      </tbody>
    </table>
  </div>
  
  <p style="text-align: center; margin-top: 2rem; color: var(--text-muted);">
    Khazaur extends the traditional AUR helper model with additional package sources. 
    For AUR-only needs, yay and paru are excellent alternatives.
  </p>
</section>

<section id="docs" class="docs">
  <h2>Documentation</h2>
  
  <div class="docs-grid">
    <div class="doc-card">
      <h3>Commands</h3>
      <p>Complete command reference and usage examples.</p>
      <a href="docs/COMMANDS.html" class="doc-link">Read →</a>
    </div>

    <div class="doc-card">
      <h3>Configuration</h3>
      <p>Customize khazaur's behavior and settings.</p>
      <a href="docs/CONFIGURATION.html" class="doc-link">Read →</a>
    </div>

    <div class="doc-card">
      <h3>Retry Logic</h3>
      <p>How khazaur handles rate limits and network errors.</p>
      <a href="docs/RETRY.html" class="doc-link">Read →</a>
    </div>
  </div>
</section>

<section class="footer-cta">
  <h2>Get started with Khazaur</h2>
  <p>Unified package management for Arch Linux</p>
  <a href="#install" class="btn btn-large">Install Now</a>
  <p class="footer-note">Made by <a href="https://github.com/os-guy-original">os-guy-original</a></p>
</section>
