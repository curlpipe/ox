cargo build --release
strip -s target/release/ox
cargo generate-rpm
cargo deb
