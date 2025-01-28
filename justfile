# Load environment variables from .env file
set dotenv-load := true

# Define a variable that holds the feature flag if USE_VENDOR_FEATURE is set
FEATURE_FLAG := if env('USE_VENDOR_FEATURE', '0') == "1" { "--features vendored" } else { "" }

# Define a variable that holds the feature flag if DISABLE_TOR is set
DISABLE_TOR := if env('DISABLE_TOR', '0') == "1" { "--features disable-tor" } else { "" }

# Environment variables:
# - USE_VENDOR_FEATURE: Set to 1 to use vendored dependencies
# - DISABLE_TOR: Set to 1 to disable tor
# - WALLET_PASSWORD: Set to auto-unlock the wallet on startup

run:
    cd harbor-ui && RUST_LOG=harbor=debug,iced_wgpu=error,wgpu_core=error,info cargo run {{FEATURE_FLAG}} {{DISABLE_TOR}}
    
watch:
    cd harbor-ui && RUST_LOG=harbor=debug,iced_wgpu=error,wgpu_core=error,info cargo watch -x "run {{FEATURE_FLAG}} {{DISABLE_TOR}}"

test:
    cargo test {{FEATURE_FLAG}}

release:
    cargo run --release {{FEATURE_FLAG}}

format-check:
    cargo fmt --all -- --check

format:
    cargo fmt --all

clippy:
    cargo clippy {{FEATURE_FLAG}} --tests -- -D warnings

ci:
    cargo fmt --all -- --check
    cargo clippy {{FEATURE_FLAG}} --tests -- -D warnings
    cargo test {{FEATURE_FLAG}}

clear-signet:
    rm -rf ~/.harbor/signet
