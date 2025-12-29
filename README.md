# Rust / Winit / Egui / Wgpu Triangle

A cross-platform [Rust](https://www.rust-lang.org/) graphics demo using [wgpu](https://wgpu.rs/) to render a spinning triangle. Supports native desktop, WebGL/WebGPU ([WASM](https://webassembly.org/)), Android, Steam Deck, and [OpenXR](https://www.khronos.org/openxr/) VR with hand tracking.

> **Related Projects:**
> - [Nightshade](https://github.com/matthewjberger/nightshade) - Game engine based on this boilerplate
> - [vulkan-example](https://github.com/matthewjberger/vulkan-example) - Vulkan version
> - [opengl-example](https://github.com/matthewjberger/opengl-example) - OpenGL version
> - [freecs](https://github.com/matthewjberger/freecs) - ECS library used by Nightshade

<img width="802" height="632" alt="native" src="https://github.com/user-attachments/assets/aaad05db-8a5b-4306-a166-2692b4e365fb" />

Other languages (experimental):
- [wgpu-example-odin](https://github.com/matthewjberger/wgpu-example-odin)
- [wgpu-example-c](https://github.com/matthewjberger/wgpu-example-c)
- [wgpu-example-zig](https://github.com/matthewjberger/wgpu-example-zig)

## Quickstart

| Platform | Command |
|----------|---------|
| Native Desktop | `cargo run -r` |
| WebGPU | `trunk serve --features webgpu --open` |
| WebGL | `trunk serve --features webgl --open` |
| Android | `just run-android DEVICE_ID` |
| Steam Deck | `just build-steamdeck && just deploy-steamdeck` |
| OpenXR VR (Desktop) | `just run-openxr` |
| OpenXR VR (Quest) | `just build-android-openxr` |

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
just build-android              # Build only (windowed app)
just build-android-all          # Build for arm64 and x64
just build-android-openxr       # Build for Meta Quest VR
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

### OpenXR VR Mode (Desktop)

Renders the spinning triangle with an infinite grid, procedural skybox, and hand tracking in VR via PCVR streaming.

**Setup:**
1. Install [SteamVR](https://store.steampowered.com/app/250820/SteamVR/)
2. Install [Virtual Desktop](https://www.vrdesktop.net/) or another OpenXR-compatible runtime
3. Start Virtual Desktop and stream your desktop to your VR headset
4. Run `just run-openxr` on your desktop

### OpenXR VR Mode (Meta Quest)

Native standalone VR for Meta Quest 2, Quest Pro, Quest 3, and Quest 3S.

**Prerequisites:**
- All Android prerequisites (see above)
- Meta Quest device with Developer Mode enabled

**Build:**
```bash
just build-android-openxr
```

This produces an APK at `target/x/release/android/app.apk`.

**Install on Quest:**
```bash
adb install -r target/x/release/android/app.apk
```

Or use [SideQuest](https://sidequestvr.com) to drag and drop the APK onto your Quest.

The app appears in your Quest library under "Unknown Sources".

<details>
<summary><strong>Technical Notes</strong></summary>

- Uses the `android-openxr` feature which combines `android` and `openxr` features
- Bundles Meta's OpenXR loader from `libs/arm64-v8a/libopenxr_loader.so`
- Manifest includes `com.oculus.intent.category.VR` for proper VR app handling
- Supports Quest hand tracking and controller input
- Requires `manifest.yaml` with `runtime_libs` configuration for library bundling
</details>

## Screenshots

<img width="1665" height="1287" alt="webgl" src="https://github.com/user-attachments/assets/d8771e73-4b0b-459a-baf2-5ce1f79f943e" />
<img width="1665" height="1287" alt="webgpu" src="https://github.com/user-attachments/assets/494f2a88-087c-4045-8433-e96f042b7988" />
