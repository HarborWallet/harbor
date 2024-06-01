# Load environment variables from .env file
set dotenv-load := true

# Define a variable that holds the feature flag if USE_VENDOR_FEATURE is set
FEATURE_FLAG := if env('USE_VENDOR_FEATURE', '0') == "1" { "--features vendored" } else { "" }

run:
    RUST_LOG=harbor=debug,iced_wgpu=error,wgpu_core=error,info cargo run {{FEATURE_FLAG}}
    
watch:
    RUST_LOG=harbor=debug,iced_wgpu=error,wgpu_core=error,info cargo watch -x "run {{FEATURE_FLAG}}"

test:
    cargo test {{FEATURE_FLAG}}

release:
    cargo run --release {{FEATURE_FLAG}}

clippy:
    cargo clippy --all-features --tests -- -D warnings

reset-db:
    diesel migration revert --all --database-url=harbor.sqlite && diesel migration run --database-url=harbor.sqlite
