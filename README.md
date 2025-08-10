# Rust / Winit / Egui / Wgpu Triangle

This project demonstrates how to setup a [rust](https://www.rust-lang.org/) project
that uses [wgpu](https://wgpu.rs/) to render a spinning triangle, supporting
both webgl and webgpu [wasm](https://webassembly.org/) as well as native.

> If you're looking to use vulkan directly instead, check out this [rust + vulkan example](https://github.com/matthewjberger/vulkan-example)

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

> Firefox is still [working on wgpu support](https://news.ycombinator.com/item?id=41157383)
> but chromium-based browsers like Brave, Vivaldi, Chrome, etc will work

## Prerequisites (web)

* [trunk](https://trunkrs.dev/)

## Screenshots

<img width="1665" height="1287" alt="webgl" src="https://github.com/user-attachments/assets/d8771e73-4b0b-459a-baf2-5ce1f79f943e" />
<img width="1665" height="1287" alt="webgpu" src="https://github.com/user-attachments/assets/494f2a88-087c-4045-8433-e96f042b7988" />
