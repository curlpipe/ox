mkdir -pv /system/packages/ox
cargo build --release
cargo install --path . --root=/system/packages/ox
