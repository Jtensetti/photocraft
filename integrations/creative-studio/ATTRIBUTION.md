# Attribution

| Path / use | Title | Author / source | License |
|---|---|---|---|
| Rust library dependencies in Cargo.toml | PhotoCraft | ArtCraft team and contributors; https://github.com/Jtensetti/photocraft at pinned revision | MIT OR Apache-2.0; texts and NOTICE in public/licenses/photocraft |
| Rust library dependencies in Cargo.toml | LightCraft | ArtCraft team and contributors; https://github.com/Jtensetti/lightcraft at pinned revision | MIT OR Apache-2.0; texts and NOTICE in public/licenses/lightcraft |
| Rust engine, timeline, codecs, render, audio and H.264 export dependencies | FilmCraft | ArtCraft team and contributors; https://github.com/Jtensetti/filmcraft at pinned revision | MIT OR Apache-2.0; texts and NOTICE in public/licenses/filmcraft |
| web/media.js and web/export.js dependency | MP4Box.js | GPAC contributors; https://github.com/gpac/mp4box.js | BSD-3-Clause; public/licenses/mp4box-LICENSE |
| web/export.js dependency | webm-muxer | Vanilagy; https://github.com/Vanilagy/webm-muxer | MIT; public/licenses/webm-muxer-LICENSE |
| web/export.js dependency | mp4-muxer | Vanilagy; https://github.com/Vanilagy/mp4-muxer | MIT; public/licenses/mp4-muxer-LICENSE |
| src/stream_audio.rs dependency | Symphonia | Symphonia contributors; locked crates from https://github.com/pdeljanov/Symphonia | MPL-2.0; full texts in generated public/licenses/rust/symphonia-* |
| public/demo/great-wave.jpg | The Great Wave off Kanagawa | Katsushika Hokusai; Metropolitan Museum of Art scan via Wikimedia Commons, https://commons.wikimedia.org/wiki/File:Tsunami_by_hokusai_19th_century.jpg | Public domain; provenance in public/demo/SOURCE.md |
| web/icons.js | Lucide icons | Lucide contributors; vendored PhotoCraft assets/icons | ISC; public/licenses/lucide-LICENSE |
| index.html, web/style.css | Editor UI and geometric text symbols | Original Creative Studio implementation; system fonts | MIT, root LICENSE |

The generated public/licenses/dependencies.json contains the locked Rust dependency
manifest and associated license texts. Upstream branding assets are excluded; the separately licensed Lucide tool icons are included with their ISC notice.

PhotoCraft text uses bundled Inter and JetBrains Mono font data under SIL OFL. Full font notices are collected under public/licenses/rust/photocraft-text-0.5.0 by the build script. The three native engines are derived command/render views of one Studio project at the pinned revisions.

VectorCraft (`da73a4613715a8e757f9c60d8d87265eebbb6a8b`) och DesignCraft (`9e4f69428f05db49fde3f695aa2431178e32ce5f`) används som oförändrade låsta Rust-motorer. Deras LICENSE/NOTICE och bundlade typsnittslicenser följer med under `licenses/`. De nya verktygsikonerna återanvänder samma Lucide/ISC-uppsättning i PhotoCrafts `assets/icons/`; inga appmärken eller upstream-skärmbilder distribueras.
