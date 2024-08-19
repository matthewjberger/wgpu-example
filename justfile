set windows-shell := ["powershell.exe"]

export RUST_LOG := "info,wgpu_core=off"
export RUST_BACKTRACE := "1"

@just:
    just --list

build:
    cargo build -r

check:
    cargo check --all --tests
    cargo fmt --all -- --check

docs $project="engine":
    cargo doc --open -p {{project}}

format:
    cargo fmt --all

fix:
    cargo clippy --all --tests --fix

lint:
    cargo clippy --all --tests -- -D warnings

run:
    cargo run -r

build-webgpu:
    trunk build --features webgl

build-webgl:
    trunk build --features webgl

run-webgl:
    trunk serve --features webgl

run-webgpu:
    trunk serve --features webgpu --open

udeps:
    cargo machete

test:
    cargo test --all -- --nocapture

@versions:
    rustc --version
    cargo fmt -- --version
    cargo clippy -- --version
