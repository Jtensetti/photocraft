# Creative Studio · gemensam kreativ arbetsyta

En lokal, tidsmedveten bild- och videoredigerare med en arbetsyta, ett Rust-projekt och kombinerbara verktygspaneler. Originalmedia stannar i webbläsaren. Projektet använder faktiska bibliotek från PhotoCraft, LightCraft och FilmCraft, utan att ändra deras källkod.

Version 0.4 ansluter originalens kommandoregister och fullständiga bild-, framkallnings- och flerspårsrenderare till samma projekt. **Alla verktyg (Ctrl/Cmd+K)** söker bland originalens verktyg och visar deras parametrar. **Fler canvasverktyg** ger direkt åtkomst till lasso, kloning, lagning, former, lokala masker och beskärning. Funktioner som kräver desktopfiler, enheter eller separata modeller markeras som otillgängliga. Se [verifierad funktionsstatus](docs/ROADMAP.md).

## Starta

Krav: Rust **1.99.0**, Node **22.12+**, npm och `wasm-bindgen-cli` **0.2.129**. Biblioteken är låsta till granskade Git-commits i `Cargo.toml`; både Rust- och npm-beroenden har låsfiler.

```sh
rustup toolchain install 1.99.0 --component rustfmt --component clippy
cargo install wasm-bindgen-cli --version 0.2.129 --locked
npm ci
npm run build:wasm
npm run dev
```

Öppna den lokala adress Vite visar. Använd aktuell Chrome eller Edge på desktop. WebCodecs-videoexport kräver HTTPS eller localhost. Bildredigering fungerar utan WebCodecs; webbläsarens tillgängliga video- och ljudkodekar avgör vilka MP4-filer som kan avkodas.

## Prova hela kedjan

1. Importera en bild, en film eller ljud. PhotoCrafts importer läser även PSD/PSB, pcraft, TIFF och stödda RAW-format; lager i ett original behålls. Webbläsarens mediestöd kompletteras av FilmCrafts avkodare. Första mediet anger projektformatet; senare medier kan ha andra bildfrekvenser och upplösningar.
2. Navigera med filmrullen, skjutreglaget eller piltangenterna. Zooma till **Bildrutor**, Shift-klicka eller dra för att välja ett intervall. Bildrutorna räknas från **0**. Mellanslag spelar/pausar.
3. Välj **Aktuell bildruta**, **Markerat intervall**, **Hela klippet** eller **Projekt · framkallning**. Sätt intervallets start med **I**, gå till sista inkluderade bildrutan och tryck **O**. Ett intervall 200–299 sparas som `[200, 300)`.
4. Måla eller sudda i PhotoCraft med hårdhet, opacitet, flöde och penntryck. Rektangulär bildmarkering begränsar nya penseldrag. Lager har egen tidsomfattning, mask och ett av 27 blandningslägen; flytta på canvasen, skala, rotera, duplicera eller ordna lagren. LightCraft erbjuder RGB-punktkurvor och reglage från den verkliga motorn för ljus, färg, HSL, svartvit mix, färggradering, effekter, vinjett, korn, detalj och kalibrering. Histogramsdata kommer från den renderade bildrutan.
5. Växla läge. Projekt, historik, spelhuvud och markering ligger kvar. FilmCraft erbjuder flera video-/ljudspår, trimning, flytt, ripple, slip/slide, hastighet, reverse, grafik, övergångar och den riktiga effektinspektören med keyframes. Dra klipphuvuden på tidslinjen för att ändra spår och starttid. **Spår** visar ljud och lagerband.
6. **Spara** lagrar projekt och historik i IndexedDB. **Öppna** återställer projektet. **Säkerhetskopia** laddar ned en `.cstudio-backup` med projekt och originalmedia, som kan öppnas på en annan enhet. Klicka på ett saknat medium för att återlänka filen.
7. Exportera aktuell bearbetad bildruta som PNG eller hela sekvensen som WebM (VP9/Opus) eller MP4 (H.264/AAC eller Opus). När browsern saknar H.264-kodning används FilmCrafts egen strömmande kodare. Exporten använder samma renderare för alla spår, bildoperationer, övergångar och ljudjusteringar.

Stillbilder har en varaktighet på en sekund vid import; FilmCraft-panelen kan ändra den till exempelvis fem sekunder utan duplicerade media eller rasterbilder. Tidslinjen kan döljas och ändra höjd. Paneler kan kombineras, dras mellan dockor, visas som flikar eller staplar, frikopplas och minimeras. Spara namngivna arbetsytor med **Spara arbetsyta**. Dessa val ligger utanför innehållshistoriken. **Spår** visar även lagrens tidsband; dra deras ändar för att ändra omfattningen. Se [konkret gränssnittslogik](docs/WORKSPACE.md).

Verktygsradens kortkommandon: **V** flytta, **M** markering, **B** pensel, **E** sudd, **T** text, **Alt+I** pipett, **H** hand och **Z** zoom. **Ctrl/Cmd+J** duplicerar lager och **Ctrl/Cmd+D** tar bort bildmarkeringen. **C** delar aktivt klipp; **I/O** behåller tidslinjens intervallval. Zoomnivån ändrar bara canvasens presentation; preview är fortfarande begränsad till 960×640.

## Verifiera

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
npx playwright install --with-deps chromium
npm run build:wasm
npm run test:browser
npm run test:workspace
npm run test:pro
npm run test:native
npm run build
npm run test:production
```

Webbläsartestet kräver `ffmpeg` och `ffprobe` med libvpx-vp9/libopus. Det importerar verklig MP4, redigerar intervall och en enskild bildruta, sparar/återöppnar, återställer backup och avkodar den faktiska exportfilen. Det kontrollerar pixelvärden, exakt antal bildrutor, bildfrekvens, ljudvaraktighet och tystnad efter klippdelning/ljud av. Testresultat och skärmbild hamnar i `test-results/`.

Det separata `test:pro` verifierar kurvor med faktiska pixlar inom rätt bildrutescope, HSL, bildmarkering, penselopacitet, blandning, skala/rotation, lagerordning, pipett, zoom/panorering och menyer genom gränssnittet. PNG-exporten avkodas och jämförs pixel för pixel med samma projektoperationer. Exempelprojektet använder en lokalt bundlad public-domain-bild; se [källa](public/demo/SOURCE.md).

`test:native` kontrollerar redigerbara pcraft-lager, faktisk lasso/filter, urklipp, gemensam historik, lokala masker, TIFF, återlänkning av lageroriginal, strömmande ljud och flerspårseffekter genom webbappen. WebM- och MP4-export avkodas med ffmpeg och jämförs med projektets pixlar och ljudmix; codec-stöd är webbläsarberoende.

## Publicera statiskt

```sh
npm run build:wasm
python3 scripts/license-notices.py
npm run build
```

Publicera **hela `dist/`** hos en statisk HTTPS-värd. Innehållet inkluderar worker, JavaScript, motor och licenstexter. Motorn paketeras i integritetskontrollerade delar om högst 20 MiB för värdar med filstorleksgränser. `base: './'` stöder domänrot och undermappar. `.js` ska levereras som JavaScript. Ingen backend, API-nyckel eller användarinloggning behövs. Öppna inte `index.html` direkt som `file://`.

GitHub Actions bygger och verifierar versionen samt laddar upp `creative-studio-web` som nedladdningsbar artefakt. Ingen produktionspublicering görs automatiskt. Återskapa gärna denna fristående mapp i ett eget `Jtensetti/creative-studio`-repository; den har en egen Cargo-workspace och kan byggas utan moderprojektet.

## Praktiska gränser

- Format: browseravkodning och originalmotorernas stödda format. Avancerade, skadade eller ovanliga codec-/containerkombinationer kan ge importfel; detta är ingen garanti för varje kameraformat. Projektets bildfrekvens är fast, källtiden samplas från originalet.
- Videoexport: nedladdning i minnet högst **120 sekunder**; **Spara video direkt till fil** stödjer längre export med interfolierade bild-/ljudköer och successiva filskrivningar i Chrome/Edge. Originalvideo läses via Blob-delar och omvandlas aldrig till PNG-sekvenser.
- Ljud: fristående WAV/MP3/FLAC/AIFF och stödda containerljud, resampling och den riktiga FilmCraft-mixern. Avkodade källfönster är begränsade; exporter och förhandslyssning använder en stereomix. Komplexa containeredit-listor behöver fortsatt formatverifiering.
- Import/export: 8-bitars sRGB via webbläsarens canvas. Framkallningen räknar i flyttal, men HDR, ICC-profiler och 16-bitars in/utdata är ännu inte bevarade genom hela kedjan.
- Renderingen körs i en worker på CPU, med förhandsvisning upp till 960×640. Native bildoperationer utvärderas vid projektupplösningen innan preview skalas ned. Uppspelning följer ljudklockan och kan hoppa över previewrutor; export renderar varje ruta.
- Media sparas lokalt per origin. Rensad webbläsardata eller byte av publiceringsadress tar bort tillgången till den lokala lagringen. Ta en portabel backup. Vid full lagring visas ett begripligt fel och osparat arbete utlöser varning vid stängning.
- Återlänkning jämför typ, byteantal och dimensioner; den beräknar ännu ingen filhash. Automatiska återställningspunkter finns. Återstart av en kraschad worker och fullständigt register för schemamigrering återstår.

Läs [arkitekturen](docs/ARCHITECTURE.md), [projekt- och kommandokontraktet](docs/CONTRACT.md), [kodinventeringen](docs/UPSTREAM-AUDIT.md) och [licens- och varumärkesöversikten](docs/LICENSING.md).
