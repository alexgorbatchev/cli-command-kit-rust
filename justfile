default: check

build:
    cargo build --all-targets

test:
    cargo test --all-targets

lint:
    cargo clippy --all-targets -- -D warnings

format:
    cargo fmt --all

coverage:
    cargo llvm-cov --all-targets --fail-under-lines 90

check:
    cargo fmt --all -- --check
    cargo clippy --all-targets -- -D warnings
    cargo test --all-targets
    cargo llvm-cov --all-targets --fail-under-lines 90
