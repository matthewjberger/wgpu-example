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

# Run the hardware ray tracing demo
run-rt:
    cargo run -r --bin rt

# Build the desktop app in OpenXR mode
build-openxr:
    cargo build -r --features openxr

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

# Serve the app with WebXR support (localhost only)
run-webxr:
    trunk serve --features webxr --open

# Serve WebXR on all network interfaces (requires HTTPS for WebXR)
run-webxr-network:
    trunk serve --features webxr --address 0.0.0.0 --port 8080

# Install cloudflared for WebXR tunnel
[windows]
init-webxr-tunnel:
    @Write-Host "Installing cloudflared..." -ForegroundColor Green
    -@winget install Cloudflare.cloudflared 2>&1 | Out-Null; if ($LASTEXITCODE -eq 0) { Write-Host "Installation successful!" -ForegroundColor Green } else { Write-Host "cloudflared may already be installed. If not, run: winget install Cloudflare.cloudflared" -ForegroundColor Yellow }
    @Write-Host ""
    @Write-Host "Note: You may need to restart your terminal for cloudflared to be in your PATH" -ForegroundColor Cyan
    @Write-Host "Then run: just run-webxr-tunnel" -ForegroundColor Green

[unix]
init-webxr-tunnel:
    @echo "Install cloudflared from: https://github.com/cloudflare/cloudflared/releases"
    @echo "Or use your package manager:"
    @echo "  - macOS: brew install cloudflared"
    @echo "  - Linux: Check the releases page for your distribution"

# Install mkcert for local HTTPS certificates
[windows]
init-webxr-https:
    @echo "Installing mkcert..."
    @echo "Choose one method:"
    @echo ""
    @echo "Option 1 - Chocolatey:"
    @echo "  choco install mkcert"
    @echo ""
    @echo "Option 2 - Scoop:"
    @echo "  scoop bucket add extras"
    @echo "  scoop install mkcert"
    @echo ""
    @echo "Option 3 - Manual:"
    @echo "  Download from: https://github.com/FiloSottile/mkcert/releases"
    @echo ""
    @echo "After installing, run: mkcert -install"

[unix]
init-webxr-https:
    @echo "Installing mkcert..."
    @echo "Visit: https://github.com/FiloSottile/mkcert"
    @echo "Then run: mkcert -install && mkcert localhost YOUR_IP"

# Serve WebXR with HTTPS using custom cert (set TRUNK_SERVE_TLS_CERT and TRUNK_SERVE_TLS_KEY)
run-webxr-https cert key:
    trunk serve --features webxr --address 0.0.0.0 --port 8443 --tls-cert {{cert}} --tls-key {{key}}

# Show local IP address for WebXR headset access
webxr-ip:
    @echo "Access from VR headset at one of these addresses:"
    @echo "(Note: WebXR requires HTTPS except for localhost)"
    @ipconfig | findstr "IPv4" || true

# Serve WebXR with Cloudflare Tunnel (easiest method for headset access)
[windows]
run-webxr-tunnel:
    @Write-Host "Starting WebXR server with Cloudflare Tunnel..." -ForegroundColor Green
    @Write-Host ""
    @Write-Host "This will:"
    @Write-Host "  1. Start the dev server"
    @Write-Host "  2. Create a temporary HTTPS tunnel"
    @Write-Host "  3. Give you a URL to use in your VR headset"
    @Write-Host ""
    @if (!(Get-Command cloudflared -ErrorAction SilentlyContinue)) { Write-Host "ERROR: cloudflared not found!" -ForegroundColor Red; Write-Host ""; Write-Host "Install it with:"; Write-Host "  winget install Cloudflare.cloudflared"; Write-Host ""; exit 1 }
    @Write-Host "Starting dev server in background..." -ForegroundColor Yellow
    @Start-Process -NoNewWindow trunk -ArgumentList "serve","--features","webxr","--address","0.0.0.0","--port","8080"
    @Write-Host "Waiting for server to start..." -ForegroundColor Yellow
    @Start-Sleep -Seconds 8
    @Write-Host ""
    @Write-Host "Creating tunnel (Press Ctrl+C to stop)..." -ForegroundColor Green
    @Write-Host ""
    cloudflared tunnel --url http://localhost:8080

[unix]
run-webxr-tunnel:
    #!/usr/bin/env bash
    set -e
    echo "Starting WebXR server with Cloudflare Tunnel..."
    echo ""
    echo "This will:"
    echo "  1. Start the dev server"
    echo "  2. Create a temporary HTTPS tunnel"
    echo "  3. Give you a URL to use in your VR headset"
    echo ""
    if ! command -v cloudflared &> /dev/null; then
        echo "ERROR: cloudflared not found!"
        echo ""
        echo "Install it from: https://github.com/cloudflare/cloudflared/releases"
        echo ""
        exit 1
    fi
    echo "Starting dev server in background..."
    trunk serve --features webxr --address 0.0.0.0 --port 8080 &
    SERVER_PID=$!
    sleep 5
    echo ""
    echo "Creating tunnel (Press Ctrl+C to stop)..."
    echo ""
    cloudflared tunnel --url http://localhost:8080 || kill $SERVER_PID

# Install Android tooling
init-android:
    rustup target add aarch64-linux-android
    rustup target add armv7-linux-androideabi
    rustup target add i686-linux-android
    rustup target add x86_64-linux-android
    cargo install --git https://github.com/rust-mobile/xbuild --bin x --locked

# Pair with Android device for wireless debugging (provide IP:port from pairing screen)
pair-android address:
    adb pair {{address}}

# Connect to Android device via wireless ADB (provide IP:port)
connect-android address:
    adb connect {{address}}

# List connected Android devices
list-android:
    adb devices

# Build the app for Android (arm64)
build-android:
    x build --release --platform android --arch arm64 --features android

# Build the app for Android (all architectures)
build-android-all: _ensure-libs-x64
    x build --release --platform android --arch arm64 --features android
    x build --release --platform android --arch x64 --features android

# Build the app for Android with OpenXR (arm64) for Meta Quest
build-android-openxr:
    x build --release --platform android --arch arm64 --features android-openxr

# Install the app on connected Android device
install-android device:
    x build --release --arch arm64 --features android --device adb:{{device}}
    adb -s {{device}} install -r target/x/release/android/app_core.apk

# Run the app on connected Android device
run-android device:
    x run --release --arch arm64 --features android --device adb:{{device}}

# Run the app on a connected Android x86_64 device or emulator
run-android-x64 device: _ensure-libs-x64
    x run --release --arch x64 --features android --device adb:{{device}}

# Start a local Android emulator with host GPU (avd is the AVD name from `just list-android-emulators`)
[unix]
start-android-emulator avd:
    "${ANDROID_HOME:?ANDROID_HOME is not set; see README emulator setup}/emulator/emulator" -avd {{avd}} -gpu host -no-snapshot

[windows]
start-android-emulator avd:
    $h = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { [Environment]::GetEnvironmentVariable('ANDROID_HOME','User') }; if (-not $h) { throw 'ANDROID_HOME is not set; see README emulator setup' }; & "$h\emulator\emulator.exe" -avd {{avd}} -gpu host -no-snapshot

# List local Android emulator AVDs
[unix]
list-android-emulators:
    "${ANDROID_HOME:?ANDROID_HOME is not set; see README emulator setup}/emulator/emulator" -list-avds

[windows]
list-android-emulators:
    $h = if ($env:ANDROID_HOME) { $env:ANDROID_HOME } else { [Environment]::GetEnvironmentVariable('ANDROID_HOME','User') }; if (-not $h) { throw 'ANDROID_HOME is not set; see README emulator setup' }; & "$h\emulator\emulator.exe" -list-avds

[unix]
[private]
_ensure-libs-x64:
    mkdir -p libs/x86_64

[windows]
[private]
_ensure-libs-x64:
    New-Item -ItemType Directory -Force libs/x86_64 | Out-Null

# Install Steam Deck tooling
init-steamdeck:
    cargo install --locked cross
    rustup toolchain install --force-non-host stable-x86_64-unknown-linux-gnu

# Build the app for Steam Deck
build-steamdeck:
    cross build --release --target x86_64-unknown-linux-gnu

# Deploy the app to Steam Deck
deploy-steamdeck:
    scp target/x86_64-unknown-linux-gnu/release/app deck@steamdeck.local:~/Downloads

# SSH into Steam Deck
steamdeck-ssh:
    ssh deck@steamdeck.local

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
