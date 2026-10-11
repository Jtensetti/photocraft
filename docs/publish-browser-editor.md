# Publish the PhotoCraft browser editor

PhotoCraft already contains a complete Rust/egui browser app in
[`apps/photocraft-web`](../apps/photocraft-web). It is **not** ArtCraft-X: this is the
layer-based photo editor (layers, PSD, painting, selections, adjustments, and export),
compiled to WebAssembly. Do not replace it with a JavaScript canvas framework.

## One-time setup: GitHub Pages

1. Open **Jtensetti/photocraft → Settings → Pages**.
2. Under **Build and deployment**, choose **Source: GitHub Actions**.
3. Merge the accompanying PR to `main`, or run **Actions → PhotoCraft website →
   Run workflow** after this workflow is on `main`.
4. After a successful deployment, visit
   **https://jtensetti.github.io/photocraft/**.
   If a custom domain is configured in GitHub Pages, use that domain instead.

On later changes to the Rust engine, web app, assets, or dependencies, a push to
`main` builds and publishes the browser editor automatically. Pull requests run
the same release build but do **not** deploy; download their
`photocraft-browser-preview` Actions artifact to test the static site locally.

The workflow uses the upstream Rust WebAssembly app, pinned Trunk 0.21.14,
`trunk build --release --locked`, and
`packaging/web/package.sh --skip-build` to enforce relative URLs and the
24 MiB WASM size ceiling. No cloud keys, accounts, database, or image-upload
service are required. The site includes the source licenses and notices.

## What the user can do

- Go to the site and choose **File → Open**, or drag and drop a photo or PSD.
- Edit with layers, masks, text, brushes, selections and adjustment layers.
- **File → Save / Save As / Export** starts a download from the browser.
- The image is processed on the user's computer. Preferences are kept locally.

**Important limitation:** the web build does **not** autosave documents or offer
crash recovery. Save or export before closing the tab. The app will warn about
unsaved edits when a browser supports `beforeunload`.

WebGPU is preferred, WebGL2 is the fallback. Hosting must use HTTPS for WebGPU
and clipboard support (GitHub Pages supplies HTTPS). You can append `?webgl`
or `?cpu` to the URL to troubleshoot graphics problems.

## Local preview

From the repository root:

```sh
rustup target add wasm32-unknown-unknown
cargo install trunk --version 0.21.14 --locked
cd apps/photocraft-web
trunk serve --release
```

Open **http://127.0.0.1:8765/**. To serve a downloaded PR artifact instead,
unzip it and run `python3 -m http.server 8765` inside the extracted
`photocraft-web-<version>/` directory.

## Acceptance smoke test

On desktop Chrome or Edge, and at least one browser using the WebGL2 fallback:

1. Open a local PNG/JPEG using the file picker.
2. Drag in another image; verify the canvas updates.
3. Add a new layer and make an edit; undo and redo it.
4. Export a PNG; reopen the download.
5. Open a real layered PSD, edit a layer, save a PSD copy and reopen it.
6. Verify that closing the tab with unsaved changes prompts a warning.

See [the upstream web-hosting documentation](../packaging/web/README.md) for
Docker, NGINX, iframes and other hosting options.
