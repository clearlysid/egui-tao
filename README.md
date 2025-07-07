# egui-tao

🚧 NOW SUCEEDED BY [tauri-plugin-egui](https://github.com/clearlysid/tauri-plugin-egui).

Check if we can render egui-based UI to a `tao` window, to eventually integrate with Tauri.

Try `cargo run --example demo` to see a window.

<img width="738" alt="demo app screenshot" src="https://github.com/user-attachments/assets/594a70ba-3f7e-474d-9e03-5855a8e64928" />

### goals + motivations

I have a (med-large) Tauri app that needs to render UI without the webview/browser overhead for some cases. `egui` seemed like it can work well for this.

So we splice across some tools and end up with:
1. `tauri` (which uses `tao` underneath) for overall application and window management
2. `egui` to render some Rust-based stateful UI to graphics
3. `wgpu`, `egui-wgpu` to paint the graphics onto a window/surface

There's been a previous attempt at this by the Tauri team, detailed [here](https://v2.tauri.app/blog/tauri-egui-0-1/) but it has since been [de-prioritized](https://github.com/tauri-apps/tauri/discussions/10089) due to a large maintainance surface-area. My approach is similar, except that I'm trying to minimize "fork maintainance" as much as possible.

This is a crude draft. Ideally, we'd contribute 2 official projects to the ecosystem:

1. An `egui-tao` crate in the [egui](https://github.com/emilk/egui) repo, similar to `egui-winit`.
2. A `tauri-plugin-egui` in the [tauri-apps/plugins-workspace](https://github.com/tauri-apps/plugins-workspace) repo.

I use the word **official** to denote support from the original project maintainers + ensuring some degree of code quality that I may not be capable of individually.

### My next steps are:
1. solve my own use-case for [Helmer](https://www.helmer.app) app.
2. check if there is interest in this project by the involved projects and communities.
3. explore ways of working on this with others if interested (assuming 2 goes well).


### update 4th july 2025

I realized that the `tauri-egui` integration goes a little deeper than _just_ rendering UI in a tauri window. It wraps `eframe`, a full application manager designed for `egui` and handles bi-directional communication between `tauri` windows and `eframe` windows, so you can use the API of your choice to manage them.

This is overkill for my use + forces us to maintain multiple "soft forks". I'm better off making a leaner integration where the `Window` handle from tauri can be used to as a render target for some `egui`, but the app/window control responsibility lies exclusively with the Tauri runtime, not eframe/egui.

### update 7th july 2025

This repo was a good learning experience and serves as a good starting point. But to ultimately get it running within `tauri`, we need a tauri plugin. So I'm stopping work on this repo and shifting focus to [tauri-plugin-egui](https://github.com/clearlysid/tauri-plugin-egui).
