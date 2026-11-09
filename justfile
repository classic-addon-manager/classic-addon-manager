# Task runner for the GPUI client. Run `just` to list recipes.

mod gen 'tools/gen.just'

# `just` defaults to `sh` on Windows; PowerShell is always there and every recipe is a plain command.
set windows-shell := ["powershell.exe", "-NoLogo", "-Command"]

[private]
default:
    @just --list --unsorted

# Build the application (debug)
build:
    cargo build

# Build the application for production (optimised, Cargo.lock enforced)
build-prod:
    cargo build --release --locked

# Build (debug) and run the application, ARGS are passed to the binary, e.g. `just run --check-updates`
run *ARGS:
    cargo run -- {{ ARGS }}

# Build (production) and run the application
run-prod *ARGS:
    cargo run --release --locked -- {{ ARGS }}

# Rebuild and restart on source changes (needs `cargo install cargo-watch`)
dev:
    cargo watch -w crates -w Cargo.toml -x run

# Type-check without producing a binary
check:
    cargo check --all-targets

# Run the test suite
test *ARGS:
    cargo test {{ ARGS }}

# Lint with clippy, warnings are errors
lint:
    cargo clippy --all-targets -- -D warnings

# Format the sources
fmt:
    cargo fmt

# Verify formatting without changing files
fmt-check:
    cargo fmt --check

# Everything CI should gate on
ci: fmt-check lint test

# Remove build artifacts
clean:
    cargo clean

# Bump the version (`just version major|minor|patch`, default patch) or set it explicitly (`just version 3.3.0`)
version target="patch":
    cargo run --quiet --manifest-path tools/setversion/Cargo.toml --target-dir target/setversion -- {{ target }}
    cargo update --workspace --offline --quiet

# --- AppImage (Linux) ---------------------------------------------------------------------------------------------

# Only evaluated on Linux: top-level variables are computed on every invocation, and the backtick needs a POSIX shell
container := if os() == "linux" { env("CONTAINER_ENGINE", `command -v podman >/dev/null 2>&1 && echo podman || echo docker`) } else { "" }
builder_image := "classic-addon-manager-appimage-builder"

# Build the AppImage into dist/ (optimised, Cargo.lock enforced)
[linux]
appimage:
    {{ container }} build -t {{ builder_image }} -f tools/appimage/Containerfile tools/appimage
    {{ container }} run --rm \
        -v "{{ justfile_directory() }}:/src:Z" \
        -v classic-addon-manager-cargo-registry:/usr/local/cargo/registry \
        -v classic-addon-manager-cargo-git:/usr/local/cargo/git \
        {{ builder_image }} bash tools/appimage/build.sh dist
