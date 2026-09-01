# Maintainer: 0sour <0sour@users.noreply.github.com>
# Contributor: 0sour <0sour@users.noreply.github.com>

pkgname=endfield-launcher
pkgver=0.1.1
pkgrel=1
pkgdesc="Endfield CN launcher for Linux (unofficial)"
arch=('x86_64')
url="https://github.com/0sour/endfield-launcher"
license=('GPL-3.0-or-later')
depends=('gtk4' 'libadwaita' 'p7zip' 'bubblewrap' 'bash')
makedepends=('cargo' 'rust' 'git')
optdepends=(
    'gamemode: run the game with gamemoderun'
    'gamescope: run the game in a micro-compositor'
)
source=("$pkgname::git+https://github.com/0sour/$pkgname.git#tag=v$pkgver")
sha256sums=('SKIP')

prepare() {
    cd "$pkgname"
    export CARGO_HOME="$srcdir/cargo-home"
    cargo fetch --locked
}

build() {
    cd "$pkgname"
    export CARGO_HOME="$srcdir/cargo-home"
    # Force static zstd compilation (avoid symbol conflicts with system lib)
    export ZSTD_SYS_USE_PKG_CONFIG=0
    # Reset CFLAGS to avoid breaking ring's assembly compilation
    export CFLAGS=""
    export CXXFLAGS=""
    export RUSTFLAGS="-C target-feature=-crt-static"
    cargo build --release --locked
}

check() {
    cd "$pkgname"
    export CARGO_HOME="$srcdir/cargo-home"
    cargo test --release --locked
}

package() {
    cd "$pkgname"

    # Binary
    install -Dm755 target/release/$pkgname "$pkgdir/usr/bin/$pkgname"

    # Desktop entry
    install -Dm644 assets/$pkgname.desktop "$pkgdir/usr/share/applications/$pkgname.desktop"

    # App icon
    install -Dm644 assets/images/icon.png \
        "$pkgdir/usr/share/icons/hicolor/512x512/apps/$pkgname.png"

    # AppStream metadata
    install -Dm644 assets/moe.launcher.$pkgname.metainfo.xml \
        "$pkgdir/usr/share/metainfo/moe.launcher.$pkgname.metainfo.xml"
}
