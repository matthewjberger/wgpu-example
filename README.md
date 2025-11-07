# Rust / Winit / Egui / Wgpu Triangle

This project demonstrates how to setup a [rust](https://www.rust-lang.org/) project
that uses [wgpu](https://wgpu.rs/) to render a spinning triangle, supporting
both webgl and webgpu [wasm](https://webassembly.org/) as well as native.

> If you're looking for a Vulkan example, check out [the vulkan-example repo](https://github.com/matthewjberger/vulkan-example)
>
> If you're looking for an OpenGL example, check out [the vulkan-example repo](https://github.com/matthewjberger/opengl-example)

<img width="802" height="632" alt="native" src="https://github.com/user-attachments/assets/aaad05db-8a5b-4306-a166-2692b4e365fb" />

## Quickstart

```
# native
cargo run -r

# webgpu
trunk serve --features webgpu --open

# webgl
trunk serve --features webgl --open
```

> All chromium-based browsers like Brave, Vivaldi, Chrome, etc support wgpu.
> Firefox also [supports wgpu](https://mozillagfx.wordpress.com/2025/07/15/shipping-webgpu-on-windows-in-firefox-141/) now starting with version `141`.

## Prerequisites (web)

* [trunk](https://trunkrs.dev/)

## Screenshots

<img width="1665" height="1287" alt="webgl" src="https://github.com/user-attachments/assets/d8771e73-4b0b-459a-baf2-5ce1f79f943e" />
<img width="1665" height="1287" alt="webgpu" src="https://github.com/user-attachments/assets/494f2a88-087c-4045-8433-e96f042b7988" />
