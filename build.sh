# Initial set-up
mkdir -p target/pkgs
rm target/pkgs/*

# Build for Linux
## Binary
cargo build --release
strip -s target/release/ox
cp target/release/ox target/pkgs/ox
## RPM
rm target/generate-rpm/*.rpm
cargo generate-rpm
cp target/generate-rpm/*.rpm target/pkgs/
## DEB
cargo deb
cp target/debian/*.deb target/pkgs/

# Build for macOS (binary)
export SDKROOT=/home/luke/dev/make/MacOSX13.3.sdk/
export PATH=$PATH:~/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin/
export CARGO_TARGET_X86_64_APPLE_DARWIN_LINKER=rust-lld
cargo zigbuild --release --target x86_64-apple-darwin
cp target/x86_64-apple-darwin/release/ox target/pkgs/ox-macos

# Build for Windows (binary)
cargo build --release --target x86_64-pc-windows-gnu
strip -s target/x86_64-pc-windows-gnu/release/ox.exe
cp target/x86_64-pc-windows-gnu/release/ox.exe target/pkgs/ox.exe

# Clean up
rm .intentionally-empty-file.o
