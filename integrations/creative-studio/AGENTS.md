# Creative Studio integration

This is an independent integration project, intended for its own repository. Its
`[workspace]` is separate from the containing PhotoCraft workspace. The user has
explicitly authorized a thin browser UI around the existing Rust engines.

Keep one authoritative Rust `Studio` in a worker. Reuse upstream paint,
composition, development and time libraries; do not create three independently
persisted Session/document instances. Frame views must not become saved projects.

Every edit has an explicit half-open clip-local scope. Preserve source offsets
and scoped operations when changing the timeline. Media stays local; never
upload it, extract a whole movie to PNGs, or read the original whole into memory.

Do not add AI generation, accounts, billing, marketplace features or ArtCraft
Services code/assets. Preserve upstream notices; no ArtCraft branding assets.

Run formatting, clippy, native tests, the WASM build, browser flow test and
production build as documented in README. Update docs/ROADMAP.md honestly when
capabilities or limitations change. Rendered pixels and actual exported video
must be tested, not merely whether buttons can be clicked.
