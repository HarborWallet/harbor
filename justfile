test:
    cargo test

run:
    RUST_LOG=harbor=debug,info cargo run

release:
    cargo run --release

clippy:
    cargo clippy --all-features --tests -- -D warnings
