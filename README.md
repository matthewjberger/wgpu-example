# Rust / Winit / Egui / Wgpu Triangle

This project demonstrates how to setup a [rust](https://www.rust-lang.org/) project
that uses [wgpu](https://wgpu.rs/) to render a spinning triangle, supporting
both webgl and webgpu [wasm](https://webassembly.org/) as well as native.

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
> but chromium-based browser like Brave, Vivaldi, Chrome, etc will run the application properly

## Prerequisites (web)

* [trunk](https://trunkrs.dev/)

## Screenshots
  
![Screenshot 2024-08-20 at 8 17 14 AM](https://github.com/user-attachments/assets/fd841943-a80b-4f27-9d9e-f85bb03d8add)
![Screenshot 2024-08-20 at 8 17 51 AM](https://github.com/user-attachments/assets/383d0122-f26d-41db-b1de-35512d697830)
