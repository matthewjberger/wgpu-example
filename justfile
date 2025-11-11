set windows-shell := ["powershell.exe"]

export RUST_LOG := "info,wgpu_core=off"
export RUST_BACKTRACE := "1"

[private]
default:
    @just --list

# Build the workspace
build:
    cargo build -r

# Check the workspace
check:
    cargo check --all --tests
    cargo fmt --all -- --check

# Show the workspace documentation
docs:
    cargo doc --open -p app

# Fix all automatically resolvable lints with clippy
fix:
    cargo clippy --all --tests --fix

# Autoformat the workspace
format:
    cargo fmt --all

# Install wasm tooling
init-wasm:
  rustup target add wasm32-unknown-unknown
  cargo install --locked trunk

# Lint the workspace
lint:
    cargo clippy --all --tests -- -D warnings

# Run the desktop app in release mode
run:
    cargo run -r

# Run the desktop app in OpenXR mode
run-openxr:
    cargo run -r --features openxr

# Build the app with wgpu + WebGL
build-webgl:
    trunk build --features webgl

# Build the app with wgpu + WebGPU
build-webgpu:
    trunk build --features webgpu

# Serve the app with wgpu + WebGL
run-webgl:
    trunk serve --features webgl

# Serve the app with wgpu + WebGPU
run-webgpu:
    trunk serve --features webgpu --open

# Install Android tooling
init-android:
    rustup target add aarch64-linux-android
    rustup target add armv7-linux-androideabi
    rustup target add i686-linux-android
    rustup target add x86_64-linux-android
    cargo install --locked xbuild

# Connect to Android device via wireless ADB (provide IP and port)
connect-android ip port="5555":
    adb connect {{ip}}:{{port}}

# List connected Android devices
list-android:
    adb devices

# Build the app for Android (arm64)
[unix]
build-android:
    x build --release --platform android --arch arm64 --features android
    cp -f target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp_core.so target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp.so

[windows]
build-android:
    x build --release --platform android --arch arm64 --features android
    Copy-Item -Force target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp_core.so target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp.so

# Build the app for Android (all architectures)
[unix]
build-android-all:
    x build --release --platform android --arch arm64 --features android
    cp -f target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp_core.so target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp.so
    x build --release --platform android --arch x64 --features android
    cp -f target/x/release/android/x64/cargo/x86_64-linux-android/release/libapp_core.so target/x/release/android/x64/cargo/x86_64-linux-android/release/libapp.so

[windows]
build-android-all:
    x build --release --platform android --arch arm64 --features android
    Copy-Item -Force target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp_core.so target/x/release/android/arm64/cargo/aarch64-linux-android/release/libapp.so
    x build --release --platform android --arch x64 --features android
    Copy-Item -Force target/x/release/android/x64/cargo/x86_64-linux-android/release/libapp_core.so target/x/release/android/x64/cargo/x86_64-linux-android/release/libapp.so

# Install the app on connected Android device
install-android device:
    x build --release --arch arm64 --features android --device adb:{{device}}
    adb -s {{device}} install -r target/x/release/android/app.apk

# Run the app on connected Android device
run-android device:
    x run --release --arch arm64 --features android --device adb:{{device}}

# Run the test suite
test:
    cargo test --all -- --nocapture

# Check for unused dependencies with cargo-machete
udeps:
  cargo machete

# Watch for changes and rebuild the app
watch $project="app":
    cargo watch -x 'run -r -p {{project}}'

# Display toolchain versions
@versions:
    rustc --version
    cargo fmt -- --version
    cargo clippy -- --version
