# Kodinventering och befintlig webbpublicering

Granskning: 2026-10-09. API-valen bygger på källkod, inte roadmap-procenttal. Commits är fixerade för reproducerbara byggnader.

| Projekt | Granskad commit | Frikopplade delar och beslut |
|---|---|---|
| [PhotoCraft](https://github.com/Jtensetti/photocraft) | `64496178d0dbac2a152a942d044ab57b747d6d54` | 833 verkliga kommandospecifikationer. 0.4 använder engine, io, format, compose, paint, doc, text, raster och geom. Derived dokument återspelar scoped operationer; originalens lager behålls. Desktop-/fil-/modellfunktioner behöver browseradaptrar. |
| [LightCraft](https://github.com/Jtensetti/lightcraft) | `c4c96ecabbbaec73af3181c01010dc5d15c7fb85` | 264 specifikationer. Engine-kommandon för develop, crop, mask, spot, curve, geometry och presets är anslutna till samma canvas och projektoperationer; pipeline utför framkallningen. Katalogens desktopfunktioner och modeller ingår inte. |
| [FilmCraft](https://github.com/Jtensetti/filmcraft) | `5231852443363f001c3f6b396dd9b1e6461ae2be` | 675 specifikationer. Native sequence är enda sparade tidsstrukturen. Engine/render/audio, codecs, media, time/project/edit och den riktiga H.264-kodaren återanvänds. Browser-SourceProvider läser original i intervall. Native desktop-output ersätts av webbens strömmande exportadapter. |

`examples/inventory.rs` härleder katalogen från de låsta motorerna. `src/native.rs` och `src/film.rs` definierar anslutningsgränserna. En temporär eller återanvänd härledd motorvy skapar inte ett separat auktoritativt projekt: bara Studio sparar och äger historik. Ingen pensel-, filter-, framkallnings-, effekt- eller H.264-algoritm har skrivits om i JavaScript.

Inventeringen omfattar även administrativa kommandon, utdatafiler, presets och modeller. Registrering är inte samma sak som individuell end-to-end-verifiering. Se ROADMAP för konkreta browsergränser.

## PhotoCrafts tidigare webbgren

[PR #1 – separat webbpublicering](https://github.com/Jtensetti/photocraft/pull/1) var **öppen och ej mergad** vid granskningen. Head var `aa0eea29ba6a717d8503c461410a9dfaa50309e1`. De kontrollerade workflow-resultaten CI, Packaging lint, Documentation och PhotoCraft website var success. Dessa resultat gäller den tidigare PR:n, inte Creative Studios nya kod.

Den tidigare grenen visar hur PhotoCraft kan publiceras självständigt. Den innehåller inte den gemensamma tidsmedvetna arbetsmiljön och används därför inte som integrationsbas. Creative Studio lämnar den grenen och samtliga upstream-motorer orörda.

## ArtCraft Services

README och LICENSE.md granskades endast som arkitektur-/licensreferens. Dess fair-source-villkor har begränsningar för konkurrerande produkter och förändringar kring betalfunktioner. **Ingen kod, UI, generativ AI, betalningsintegration eller användarhantering har kopierats därifrån.** Den lokala integrationsimplementationen bygger endast på de tre öppna motorbiblioteken och separat licensierade webbberoenden.

## GitHub-leverans

GitHub-anslutningen i denna session hade funktioner för branches, commits och PR:er men ingen funktion för att skapa repository. `Jtensetti/creative-studio` kunde inte hämtas. Den fristående mappen levereras därför för granskning under `integrations/creative-studio` i en separat PhotoCraft-branch, tillsammans med en avgränsad CI-workflow. PhotoCraft blir inte huvudmotor; mappen har en egen Cargo-workspace och ska kunna flyttas oförändrad till ett eget integrationsrepository.
