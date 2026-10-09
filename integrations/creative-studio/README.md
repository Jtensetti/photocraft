# Creative Studio · gemensam kreativ arbetsyta

En lokal, tidsmedveten bild- och videoredigerare med en arbetsyta, ett Rust-projekt och kombinerbara verktygspaneler. Originalmedia stannar i webbläsaren. Projektet använder faktiska bibliotek från PhotoCraft, LightCraft och FilmCraft, utan att ändra deras källkod.

Detta är en fungerande vertikal första leverans, inte de tre fullständiga programmens samtliga verktyg. Se [funktionsstatus och nästa steg](docs/ROADMAP.md).

## Starta

Krav: Rust **1.99.0**, Node **22.12+**, npm och `wasm-bindgen-cli` **0.2.105**. Biblioteken är låsta till granskade Git-commits i `Cargo.toml`; både Rust- och npm-beroenden har låsfiler.

```sh
rustup toolchain install 1.99.0 --component rustfmt --component clippy
cargo install wasm-bindgen-cli --version 0.2.105 --locked
npm ci
npm run build:wasm
npm run dev
```

Öppna den lokala adress Vite visar. Använd aktuell Chrome eller Edge på desktop. WebCodecs-videoexport kräver HTTPS eller localhost. Bildredigering fungerar utan WebCodecs; webbläsarens tillgängliga video- och ljudkodekar avgör vilka MP4-filer som kan avkodas.

## Prova hela kedjan

1. Skapa en tom bild eller importera PNG, JPEG, WebP eller en MP4 med konstant bildfrekvens. Första mediet anger projektets upplösning och bildfrekvens; senare videor måste ha samma bildfrekvens.
2. Navigera med filmrullen, skjutreglaget eller piltangenterna. Zooma till **Bildrutor**, Shift-klicka eller dra för att välja ett intervall. Bildrutorna räknas från **0**. Mellanslag spelar/pausar.
3. Välj **Aktuell bildruta**, **Markerat intervall**, **Hela klippet** eller **Projekt · framkallning**. Sätt intervallets start med **I**, gå till sista inkluderade bildrutan och tryck **O**. Ett intervall 200–299 sparas som `[200, 300)`.
4. Måla eller sudda i PhotoCraft. Lager har egen omfattning, synlighet och opacitet. Skapa textlager, dra en rektangulär mask eller förflytta det valda lagret i X/Y. I LightCraft kan exponering, kontrast, högdagrar, skuggor, temperatur, nyans och mättnad ändras för samma omfattning.
5. Växla läge. Projekt, historik, spelhuvud och markering ligger kvar. FilmCraft-panelen erbjuder delning, riktig trimning, omordning, ljudvolym/av/på och stillbildens varaktighet.
6. **Spara** lagrar projekt och historik i IndexedDB. **Öppna** återställer projektet. **Säkerhetskopia** laddar ned en `.cstudio-backup` med projekt och originalmedia, som kan öppnas på en annan enhet. Klicka på ett saknat medium för att återlänka filen.
7. Exportera aktuell bearbetad bildruta som PNG eller hela sekvensen som WebM med VP9 och, när tillgängligt, Opus-ljud. Exporten renderar varje bildruta i ordning med explicit tidsstämpel.

Stillbilder har en varaktighet på en sekund vid import; FilmCraft-panelen kan ändra den till exempelvis fem sekunder utan duplicerade media eller rasterbilder. Tidslinjen kan döljas och ändra höjd. Paneler kan kombineras, dras mellan dockor, visas som flikar eller staplar, frikopplas och minimeras. Spara namngivna arbetsytor med **Spara arbetsyta**. Dessa val ligger utanför innehållshistoriken. **Spår** visar även lagrens tidsband; dra deras ändar för att ändra omfattningen. Se [konkret gränssnittslogik](docs/WORKSPACE.md).

## Verifiera

```sh
cargo fmt --all --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
npx playwright install --with-deps chromium
npm run build:wasm
npm run test:browser
npm run test:workspace
npm run build
npm run test:production
```

Webbläsartestet kräver `ffmpeg` och `ffprobe` med libvpx-vp9/libopus. Det importerar verklig MP4, redigerar intervall och en enskild bildruta, sparar/återöppnar, återställer backup och avkodar den faktiska exportfilen. Det kontrollerar pixelvärden, exakt antal bildrutor, bildfrekvens, ljudvaraktighet och tystnad efter klippdelning/ljud av. Testresultat och skärmbild hamnar i `test-results/`.

Testets MP4 använder **VP9/Opus**. H.264/AAC i vanlig Chrome/Edge hanteras genom webbläsarens avkodning och kontroller av WebCodecs-stöd, men har **inte verifierats** i den medföljande Chromium-miljön. Det är en separat kvarvarande verifiering.

## Publicera statiskt

```sh
npm run build:wasm
python3 scripts/license-notices.py
npm run build
```

Publicera **hela `dist/`** hos valfri statisk HTTPS-värd. Innehållet inkluderar worker, JavaScript, WASM och licenstexter. `base: './'` stöder både domänrot och en undermapp, exempelvis `/creative-studio/`. Servern måste leverera `.wasm` som `application/wasm` och `.js` som JavaScript. Ingen backend, API-nyckel eller användarinloggning behövs. Öppna inte `index.html` direkt som `file://`.

GitHub Actions bygger och verifierar versionen samt laddar upp `creative-studio-web` som nedladdningsbar artefakt. Ingen produktionspublicering görs automatiskt. Återskapa gärna denna fristående mapp i ett eget `Jtensetti/creative-studio`-repository; den har en egen Cargo-workspace och kan byggas utan moderprojektet.

## Praktiska gränser

- Video: läsbar MP4, konstant bildfrekvens, samma bildfrekvens genom projektet. Andra upplösningar anpassas proportionellt till projektets canvas.
- Videoexport: nedladdning i minnet högst **120 sekunder**; **Spara video direkt till fil** stödjer längre export med interfolierade bild-/ljudköer och successiva filskrivningar i Chrome/Edge. Originalvideo läses via Blob-delar och omvandlas aldrig till PNG-sekvenser.
- Ljudexport: MP4 med ett AAC- eller Opus-spår, **48 kHz**, mono/stereo och en enkel edit-lista. Osupporterade format ger ett fel före videoexport; ljud av ger bildexport utan detta spår.
- Import/export: 8-bitars sRGB via webbläsarens canvas. Framkallningen räknar i flyttal, men HDR, ICC-profiler och 16-bitars in/utdata är ännu inte bevarade genom hela kedjan.
- Renderingen körs i en worker på CPU, med förhandsvisning upp till 960×640. Förhandsvisning kan hoppa över bildrutor på långsam hårdvara och följer videons medieklocka; export hoppar aldrig över bildrutor.
- Media sparas lokalt per origin. Rensad webbläsardata eller byte av publiceringsadress tar bort tillgången till den lokala lagringen. Ta en portabel backup. Vid full lagring visas ett begripligt fel och osparat arbete utlöser varning vid stängning.
- Återlänkning jämför typ, byteantal och dimensioner; den beräknar ännu ingen filhash. Automatiska återställningspunkter finns. Återstart av en kraschad worker och fullständigt register för schemamigrering återstår.

Läs [arkitekturen](docs/ARCHITECTURE.md), [projekt- och kommandokontraktet](docs/CONTRACT.md), [kodinventeringen](docs/UPSTREAM-AUDIT.md) och [licens- och varumärkesöversikten](docs/LICENSING.md).
