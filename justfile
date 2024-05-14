test:
    cargo test

run:
    RUST_LOG=harbor=debug,info cargo run

release:
    cargo run --release

clippy:
    cargo clippy --all-features --tests -- -D warnings

reset-db:
    diesel migration revert --all --database-url=harbor.sqlite && diesel migration run --database-url=harbor.sqlite
