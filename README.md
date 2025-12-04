# Rust / Winit / Egui / Wgpu Triangle

A cross-platform [Rust](https://www.rust-lang.org/) graphics demo using [wgpu](https://wgpu.rs/) to render a spinning triangle. Supports native desktop, WebGL/WebGPU ([WASM](https://webassembly.org/)), Android, Steam Deck, and [OpenXR](https://www.khronos.org/openxr/) VR with hand tracking.

> **Related Projects:**
> - [vulkan-example](https://github.com/matthewjberger/vulkan-example) - Vulkan version
> - [opengl-example](https://github.com/matthewjberger/opengl-example) - OpenGL version
> - [Nightshade](https://matthewberger.dev/nightshade) - Game engine based on this boilerplate
> - [freecs](https://github.com/matthewjberger/freecs) - ECS library used by Nightshade

<img width="802" height="632" alt="native" src="https://github.com/user-attachments/assets/aaad05db-8a5b-4306-a166-2692b4e365fb" />

## Quickstart

| Platform | Command |
|----------|---------|
| Native Desktop | `cargo run -r` |
| WebGPU | `trunk serve --features webgpu --open` |
| WebGL | `trunk serve --features webgl --open` |
| Android | `just run-android DEVICE_ID` |
| Steam Deck | `just build-steamdeck && just deploy-steamdeck` |
| OpenXR VR | `just run-openxr` |

## Platform Setup

### Web (WebAssembly)

**Prerequisites:** [trunk](https://trunkrs.dev/)

**Browser Support:** All Chromium-based browsers (Chrome, Brave, Vivaldi) support WebGPU. Firefox supports WebGPU starting with version 141 ([announcement](https://mozillagfx.wordpress.com/2025/07/15/shipping-webgpu-on-windows-in-firefox-141/)).

### Android

**Prerequisites:**
- [xbuild](https://github.com/rust-mobile/xbuild)
- Android SDK and NDK
- Connected Android device or emulator (API level 24+)

**First-time setup:**
```bash
just init-android
```

**Build and run:**
```bash
just list-android          # Find your device ID
just run-android DEVICE_ID # e.g., just run-android RFCY61DZZKT
```

Connect via USB with USB debugging enabled, or use wireless debugging (see below).

The build uses `--features android` which enables wgpu's Vulkan backend.

<details>
<summary><strong>Wireless Debugging Setup</strong></summary>

1. Enable Developer options: **Settings > About phone > tap Build number 7 times**
2. Disable auto-blocker if present (Samsung): **Settings > Security > Auto Blocker**
3. Enable wireless debugging: **Settings > Developer options > Wireless debugging**
4. Tap **Pair device with pairing code** and note the IP:port
5. Pair and connect:
   ```bash
   just pair-android 192.168.1.100:37000  # Enter pairing code when prompted
   just list-android                       # Get device ID
   just run-android DEVICE_ID
   ```
</details>

<details>
<summary><strong>Additional Android Commands</strong></summary>

```bash
just build-android              # Build only
just build-android-all          # Build for arm64 and x64
just install-android DEVICE_ID  # Install without running
just connect-android IP:PORT    # Connect over wireless ADB
just list-android               # List connected devices
```
</details>

### Steam Deck

**Prerequisites:**
- [cross](https://github.com/cross-rs/cross)
- Docker (for cross-compilation)

**First-time setup:**
```bash
just init-steamdeck
```

**Build and deploy:**
```bash
just build-steamdeck   # Cross-compiles to target/x86_64-unknown-linux-gnu/release/app
just deploy-steamdeck  # Transfers to steamdeck.local:~/Downloads
```

**Run on Steam Deck:**
```bash
just steamdeck-ssh
cd ~/Downloads && ./app
```

The `Cross.toml` file configures system libraries for graphics and windowing support.

### OpenXR VR Mode

Renders the spinning triangle with an infinite grid, procedural skybox, and hand tracking in VR.

**Setup:**
1. Install [SteamVR](https://store.steampowered.com/app/250820/SteamVR/)
2. Install [Virtual Desktop](https://www.vrdesktop.net/) or another OpenXR-compatible runtime
3. Start Virtual Desktop and stream your desktop to your VR headset
4. Run `just run-openxr` on your desktop

## Screenshots

<img width="1665" height="1287" alt="webgl" src="https://github.com/user-attachments/assets/d8771e73-4b0b-459a-baf2-5ce1f79f943e" />
<img width="1665" height="1287" alt="webgpu" src="https://github.com/user-attachments/assets/494f2a88-087c-4045-8433-e96f042b7988" />
