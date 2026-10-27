run *args:
    cargo run -- {{args}}

run-ai *args:
    AGENT=1 cargo run -- {{args}}

test:
    cargo test --all-targets

fmt:
    cargo fmt --package shazam-cli

lint:
    cargo fmt --package shazam-cli -- --check
    cargo clippy --all-targets -- -D warnings

check: lint
    cargo llvm-cov --all-targets --fail-under-lines 90

build:
    cargo build --release --bin shazam-cli
    mkdir -p bin
    cp target/release/shazam-cli bin/shazam-cli
