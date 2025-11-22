# Maintainer: os-guy-original <https://github.com/os-guy-original>
pkgname=khazaur
pkgver=0.1.0
pkgrel=1
pkgdesc="Unified package manager for Arch Linux - AUR, repos, Flatpak, Snap, and Debian packages"
arch=('x86_64')
url="https://github.com/os-guy-original/khazaur"
license=('GPL3')
depends=('pacman' 'sudo')
makedepends=('rust' 'cargo' 'libgit2' 'openssl' 'zlib')
optdepends=(
    'flatpak: for Flatpak application support'
    'snapd: for Snap package support'
    'debtap: for Debian package conversion'
    'git: for faster AUR package downloads'
)
source=("$pkgname-$pkgver.tar.gz::$url/archive/v$pkgver.tar.gz")
sha256sums=('SKIP')

build() {
    cd "$pkgname-$pkgver"
    cargo build --release
}

check() {
    cd "$pkgname-$pkgver"
    cargo test --release
}

package() {
    cd "$pkgname-$pkgver"
    
    # Install binary
    install -Dm755 "target/release/$pkgname" "$pkgdir/usr/bin/$pkgname"
    
    # Install documentation
    install -Dm644 README.md "$pkgdir/usr/share/doc/$pkgname/README.md"
    install -Dm644 docs/COMMANDS.md "$pkgdir/usr/share/doc/$pkgname/COMMANDS.md"
    install -Dm644 docs/CONFIGURATION.md "$pkgdir/usr/share/doc/$pkgname/CONFIGURATION.md"
    install -Dm644 docs/RETRY.md "$pkgdir/usr/share/doc/$pkgname/RETRY.md"
    
    # Install license
    install -Dm644 LICENSE "$pkgdir/usr/share/licenses/$pkgname/LICENSE"
}
