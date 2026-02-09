# mu-epub justfile

# Run all checks
all:
    just fmt-check
    just lint
    just check
    just render-all
    just check-no-std
    just check-no-std-layout
    just test
    just test-ignored
    just doc-check
    just cli-check

# Format code
fmt:
    cargo fmt --all

# Check formatting without changes
fmt-check:
    cargo fmt --all -- --check

# Lint with clippy
lint:
    cargo clippy --all-features -- -D warnings

# Lint no_std (no default features)
lint-no-std:
    cargo clippy --no-default-features -- -D warnings

# Lint embedded mode (alloc only, no std)
lint-embedded:
    cargo clippy --no-default-features --features alloc -- -D warnings

# Check all features
check:
    cargo check --all-features

# Check split render crates
render-check:
    cargo check -p mu-epub-render -p mu-epub-embedded-graphics

# Lint split render crates
render-lint:
    cargo clippy -p mu-epub-render -p mu-epub-embedded-graphics --all-targets -- -D warnings

# Test split render crates
render-test:
    cargo test -p mu-epub-render -p mu-epub-embedded-graphics

# Run all split render crate checks
render-all:
    just render-check
    just render-lint
    just render-test

# Check no_std (no default features)
check-no-std:
    cargo check --no-default-features

# Run tests
test:
    cargo test --all-features

# Run ignored tests
test-ignored:
    cargo test --all-features -- --ignored

# Run tests with output
test-verbose:
    cargo test --all-features -- --nocapture

# Run allocation count tests
test-alloc:
    cargo test --all-features --test allocation_tests -- --nocapture

# Run embedded mode tests with tiny budgets
test-embedded:
    cargo test --all-features --test embedded_mode_tests -- --nocapture

# Verify benchmark fixture corpus integrity
bench-fixtures-check:
    sha256sum -c tests/fixtures/bench/SHA256SUMS

# Build docs
doc:
    cargo doc --all-features --no-deps

# Build docs and fail on warnings
doc-check:
    RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps

# Build docs and open locally
doc-open:
    cargo doc --all-features --no-deps --open

# Build release
build:
    cargo build --release --all-features

# Check CLI build
cli-check:
    cargo check --features cli --bin mu-epub

# Run CLI
cli *args:
    cargo run --features cli --bin mu-epub -- {{args}}

# Bootstrap external test datasets (not committed)
dataset-bootstrap:
    ./scripts/datasets/bootstrap.sh

# Bootstrap with explicit Gutenberg IDs (space-separated)
dataset-bootstrap-gutenberg *ids:
    ./scripts/datasets/bootstrap.sh {{ids}}

# List all discovered dataset EPUB files
dataset-list:
    ./scripts/datasets/list_epubs.sh

# Validate all dataset EPUB files
dataset-validate:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh --expectations scripts/datasets/expectations.tsv

# Validate all dataset EPUB files in strict mode (warnings fail too)
dataset-validate-strict:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh --strict --expectations scripts/datasets/expectations.tsv

# Validate against expectation manifest (default mode)
dataset-validate-expected:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh --expectations scripts/datasets/expectations.tsv

# Validate against expectation manifest in strict mode
dataset-validate-expected-strict:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh --strict --expectations scripts/datasets/expectations.tsv

# Raw validate mode (every file must pass validation)
dataset-validate-raw:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh

# Raw strict validate mode (warnings fail too)
dataset-validate-raw-strict:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh --strict

# Validate a small, CI-ready mini corpus from a manifest
dataset-validate-mini:
    @cargo build --features cli --bin mu-epub
    ./scripts/datasets/validate.sh --manifest tests/datasets/manifest-mini.tsv

# Run benchmarks and save latest CSV report
bench:
    @mkdir -p target/bench
    @cargo bench --bench epub_bench --all-features | tee target/bench/latest.csv

# Check no_std + layout
check-no-std-layout:
    cargo check --no-default-features --features layout

# MSRV check (matches Cargo.toml rust-version)
check-msrv:
    cargo +1.85.0 check --all-features

# Clean build artifacts
clean:
    cargo clean
