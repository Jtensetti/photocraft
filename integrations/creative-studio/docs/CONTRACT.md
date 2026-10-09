# Gemensamt projekt- och verktygskontrakt

## Persistens, schema 1

`Studio.save()` returnerar JSON med `format: "creative-studio"`, `project`, `undo` och `redo`. `project.schema_version` är 1. Nyare/okända scheman avvisas; ingen tyst nedgradering sker. `open()` validerar aktuellt projekt och historik innan någon state ändras. Gränsen för importerad projekt-JSON är 32 MB. Aktuellt projekt får innehålla högst 15 MB serialiserad metadata/operationer och den gemensamma historiken högst 15 MB; äldre snapshots tas bort automatiskt vid innehållsändringar.

| Fält | Betydelse |
|---|---|
| `assets[]` | Stabilt id, namn, image/video/blank, dimensioner, byteantal, eventuell käll-fps. Mediapixlar finns separat i IndexedDB med samma id. |
| `clips[]` | Stabilt id, asset-id, antal bildrutor, källans in-punkt i projektbildrutor, ljud av/på. Arrayordningen ger den första versionens enda sekventiella spår. |
| `fps` | FilmCraft `FrameRate`: rationella heltal `num` och `den`, 1–120 fps. |
| `layers[]` | Id, namn, synlighet, opacitet, scope och ordnade pensel-/suddoperationer. |
| `adjustments[]` | Ett scope och partiella fotografiska parametrar. Samma scope uppdaterar samma operation; ingen kopia per bildruta. |
| `workspace` | Läge, omfattning, spelhuvud, vald markering/lager, panelsynlighet och tidslinjehöjd. |
| `width`, `height` | Gemensam renderingsstorlek. Media anpassas proportionellt med transparenta marginaler. |

Ett scope är `{clip_id, start, end}` med `0 <= start < end <= clip.frames`. **Start inkluderas; end exkluderas.** UI visar den sista inkluderade bildrutan som `end - 1`. Identiteten hos en källbildruta är asset-id plus källindex; den globala bildrutan är dess tidsposition i den aktuella sekvensen.

Penselpunkter är normaliserade `[x, y, pressure]`, färg är raka RGBA-komponenter 0–1 och penselstorlek är andel av canvasbredden. Lager renderas i arrayordning ovanpå framkallat original. Framkallningsoperationer i överlappande intervall evalueras i operationsordning; sista explicit satta värdet per parameter vinner. Temperatur/nyans/mättnad/etc är samma parametrar för samtliga bildrutor i omfattningen, utan automatisk variation.

## WASM-API

- `execute(command, paramsJson) -> inspectJson`: atomiskt kommandoutförande. Okänt kommando, ogiltiga värden eller intervall ger fel och bevarar föregående projekt.
- `inspect() -> inspectJson`: projekt, total_frames, active_clip, local_frame, source_seconds, sekunder/ticks, aktiva framkallningsvärden och undo/redo-flaggor.
- `render(width, height, global_frame, rgba8) -> rgba8`: evaluerar samma projekt vid tiden utan att ändra spelhuvud eller historik.
- `save() -> projectJson`, `open(projectJson) -> inspectJson`.

| Kommando | Parametrar |
|---|---|
| `project.new` | valfritt name, width, height, fps |
| `project.rename` | name |
| `asset.add` | asset, frames; lägger till originalreferens och ett klipp |
| `seek` | global frame; klamras till tidslinjen |
| `selection.set` | clip_id, lokal start, exklusiv end |
| `view.set` | valfria mode, scope, selected_layer, left_visible, right_visible, timeline_visible, timeline_height |
| `develop.set` | values: partiella exposure/contrast/highlights/shadows/temperature/tint/saturation; omfattning från workspace |
| `layer.new` | valfritt name; omfattning från workspace |
| `layer.stroke` | stroke med points/color/size/erase; skapar ett matchande lager om valt lager har annan omfattning |
| `layer.set` | id, valfria visible/opacity |
| `layer.delete` | id |
| `clip.split` | delar aktivt klipp vid spelhuvudet och bevarar redigeringarnas tider |
| `clip.mute` | id; växlar ljud av/på |
| `clip.duration` | id, frames; endast stillbild/blank |
| `undo`, `redo` | inga |

View, seek och selection skapar ingen innehållshistorik. Innehållskommandon gör det. Sparade snapshots innehåller också workspace-state; undo kan därför återställa arbetsytans dåvarande val.

Workern använder meddelanden `{id, op, ...params}` och svarar `{id, result}` eller `{id, error}`. Renderingsbuffertar överförs som transferable ArrayBuffer. Inga medie-original skickas in i WASM. UI kan aldrig mutera motorns projekt genom att ändra ett `inspect`-svar.

Backupformatet har åtta ASCII-byte `CSTUDIO1`, fyra byte big-endian headerlängd, UTF-8 JSON-header `{json, media:[{id,size,type}]}` och därefter respektive originalmedia. Längder valideras före Blob-slicing. Backupen kräver samtliga original; saknade original måste återlänkas först.
