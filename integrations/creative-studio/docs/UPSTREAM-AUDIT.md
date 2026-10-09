# Kodinventering och befintlig webbpublicering

Granskning: 2026-10-09. API-valen bygger på källkod, inte roadmap-procenttal. Commits är fixerade för reproducerbara byggnader.

| Projekt | Granskad commit | Frikopplade delar och beslut |
|---|---|---|
| [PhotoCraft](https://github.com/Jtensetti/photocraft) | `64496178d0dbac2a152a942d044ab57b747d6d54` | `crates/engine/src/lib.rs`: Session, dokumentlista och egen undo; `commands.rs`: kommandospecifikationer; `apps/photocraft-web`: browseradapter. `crates/paint` och `crates/compose` är fristående och används direkt. `doc` är temporär renderingsvy. 0.3 återanvänder även `text`, `Affine`, 27 `BlendMode`, BrushSettings och kompositionens lagermasker; vector/PSD/RAW/GPU-adaptrar återstår. |
| [LightCraft](https://github.com/Jtensetti/lightcraft) | `c4c96ecabbbaec73af3181c01010dc5d15c7fb85` | `crates/engine/src/lib.rs`: Session äger Catalog och användarval; `crates/pipeline/src/lib.rs`: render/render_cached/RenderRequest; `develop`: DevelopSettings; `color`: färgrymder/transfer. Pipeline/develop/color/raster används direkt; 0.3 exponerar stödda ControlSpec, RGB-kurvor, HSL, grading, effekter och detaljreglage. Katalog, preset-/maskkommandon, RAW och GPU kräver egna adapters senare. |
| [FilmCraft](https://github.com/Jtensetti/filmcraft) | `5231852443363f001c3f6b396dd9b1e6461ae2be` | `crates/engine/src/lib.rs`: Session/Arc<Project>/kommandoregister; `project`: items/sequences/tracks; `render`: render_sequence/render_clip/SourceProvider; `export`: encoders, export och OutputSink. `time`, `project` och `edit` används nu: trimning körs genom en tillfällig FilmCraft-vy, utan separat sparat projekt. Full render/export kan återanvändas med härledd flerspårsvy och browser-SourceProvider; den adaptern återstår. |

De fullständiga Session-API:erna är användbara för respektive självständiga produkt, men äger olika persistenta tillstånd. Att bara koppla deras `execute()` till tre paneler skulle inte uppfylla en gemensam projektmodell. Integrationslagret har därför ett mindre gemensamt kommandoregister och återanvänder motorfunktionerna nedanför Session. Ingen pensel-, kompositions- eller fotografisk algoritm har skrivits om i JavaScript.

Plattformsberoende media, filåtkomst och codec-registrering går via webbläsarens Blob/IndexedDB/video/WebCodecs i första versionen. Native filesystem-export och desktop-egui-paneler följer inte automatiskt med till ett tunt webbskal. Bild- och ljuddata passerar genom samma lokala renderingstjänst och sparas inte som separata bildrutedokument.

## PhotoCrafts tidigare webbgren

[PR #1 – separat webbpublicering](https://github.com/Jtensetti/photocraft/pull/1) var **öppen och ej mergad** vid granskningen. Head var `aa0eea29ba6a717d8503c461410a9dfaa50309e1`. De kontrollerade workflow-resultaten CI, Packaging lint, Documentation och PhotoCraft website var success. Dessa resultat gäller den tidigare PR:n, inte Creative Studios nya kod.

Den tidigare grenen visar hur PhotoCraft kan publiceras självständigt. Den innehåller inte den gemensamma tidsmedvetna arbetsmiljön och används därför inte som integrationsbas. Creative Studio lämnar den grenen och samtliga upstream-motorer orörda.

## ArtCraft Services

README och LICENSE.md granskades endast som arkitektur-/licensreferens. Dess fair-source-villkor har begränsningar för konkurrerande produkter och förändringar kring betalfunktioner. **Ingen kod, UI, generativ AI, betalningsintegration eller användarhantering har kopierats därifrån.** Den lokala integrationsimplementationen bygger endast på de tre öppna motorbiblioteken och separat licensierade webbberoenden.

## GitHub-leverans

GitHub-anslutningen i denna session hade funktioner för branches, commits och PR:er men ingen funktion för att skapa repository. `Jtensetti/creative-studio` kunde inte hämtas. Den fristående mappen levereras därför för granskning under `integrations/creative-studio` i en separat PhotoCraft-branch, tillsammans med en avgränsad CI-workflow. PhotoCraft blir inte huvudmotor; mappen har en egen Cargo-workspace och ska kunna flyttas oförändrad till ett eget integrationsrepository.
