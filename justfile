# List available recipes.
default:
    @just --list

# Run the local Rust, release-evidence and site checks.
check: fmt-check lint test docs site
    python3 tools/test_release_evidence.py

# Format Rust sources.
fmt:
    cargo fmt --all

# Check Rust formatting.
fmt-check:
    cargo fmt --all --check

# Lint all targets with every Cargo feature enabled.
lint:
    cargo clippy --locked --all-features --all-targets -- -D warnings

# Test both the default library and all optional features.
test:
    cargo test --locked
    cargo test --locked --all-features

# Build the full Rust API reference.
docs:
    RUSTDOCFLAGS="-D warnings" cargo doc --locked --all-features --no-deps

# Build the Zola site and check its internal links.
site:
    zola --root site build
    zola --root site check --skip-external-links

# Preview the Zola site locally with live reload.
serve:
    zola --root site serve

# Print release evidence as JSON. Pass --peers to include interoperability checks.
[positional-arguments]
evidence *args:
    python3 tools/release_evidence.py "$@"

# Run the optional KNX CLI from the checkout.
[positional-arguments]
cli *args:
    cargo run --quiet --locked --features cli --bin knx -- "$@"
