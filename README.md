# Rust / Winit / Egui / Wgpu Triangle

This project demonstrates how to setup a [rust](https://www.rust-lang.org/) project
that uses [wgpu](https://wgpu.rs/) to render a spinning triangle, supporting
both webgl and webgpu [wasm](https://webassembly.org/), native desktop, and Android.

It also includes an [OpenXR](https://www.khronos.org/openxr/) VR mode with hand tracking, procedural skybox, and infinite grid.

> If you're looking for a Vulkan example, check out [the vulkan-example repo](https://github.com/matthewjberger/vulkan-example)
>
> If you're looking for an OpenGL example, check out [the opengl-example repo](https://github.com/matthewjberger/opengl-example)

<img width="802" height="632" alt="native" src="https://github.com/user-attachments/assets/aaad05db-8a5b-4306-a166-2692b4e365fb" />

## Quickstart

```bash
# native
cargo run -r

# webgpu
trunk serve --features webgpu --open

# webgl
trunk serve --features webgl --open

# android
just run-android DEVICE_ID

# OpenXR VR mode
just run-openxr
```

> All chromium-based browsers like Brave, Vivaldi, Chrome, etc support wgpu.
> Firefox also [supports wgpu](https://mozillagfx.wordpress.com/2025/07/15/shipping-webgpu-on-windows-in-firefox-141/) now starting with version `141`.

## OpenXR VR Mode

The OpenXR VR mode renders a spinning triangle, infinite grid, and procedural skybox in virtual reality with hand tracking.

### Setup

1. Install [SteamVR](https://store.steampowered.com/app/250820/SteamVR/)
2. Install [Virtual Desktop](https://www.vrdesktop.net/) (or another OpenXR-compatible VR runtime)
3. Start Virtual Desktop and stream your desktop to your VR headset
4. On your desktop, run `just run-openxr`
5. The application will appear in VR

## Prerequisites (web)

* [trunk](https://trunkrs.dev/)

## Prerequisites (android)

* [xbuild](https://github.com/rust-mobile/xbuild)
* Android SDK and NDK
* A connected Android device or emulator

### Android Build Instructions

1. Install Android tooling (first time only):
   ```bash
   just init-android
   ```
   This installs the Android Rust toolchains and xbuild.

2. Connect your Android device via USB and enable USB debugging, or start an Android emulator.

3. Find your device ID:
   ```bash
   just list-android
   ```
   This will show connected devices like:
   ```
   List of devices attached
   RFCY61DZZKT     device
   ```

4. Build and run on your device (replace `DEVICE_ID` with your device from step 3):
   ```bash
   just run-android DEVICE_ID
   ```
   Example: `just run-android RFCY61DZZKT`

### Additional Android Commands

```bash
# Build only (without running)
just build-android

# Build for all architectures (arm64 and x64)
just build-android-all

# Install without running
just install-android DEVICE_ID

# Connect to device over wireless ADB
just connect-android 192.168.1.100

# List all connected devices
just list-android
```

The Android build uses the `--features android` flag which enables wgpu's Vulkan backend. Requires Android API level 24 or higher.

## Screenshots

<img width="1665" height="1287" alt="webgl" src="https://github.com/user-attachments/assets/d8771e73-4b0b-459a-baf2-5ce1f79f943e" />
<img width="1665" height="1287" alt="webgpu" src="https://github.com/user-attachments/assets/494f2a88-087c-4045-8433-e96f042b7988" />
