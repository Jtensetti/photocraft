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

View, seek och selection skapar ingen innehållshistorik. Innehållskommandon gör det. Sparade snapshots innehåller också workspace-state; undo/redo bevarar dock aktuell presentation och reparerar ogiltiga innehållsval.

Workern använder meddelanden `{id, op, ...params}` och svarar `{id, result}` eller `{id, error}`. Renderingsbuffertar överförs som transferable ArrayBuffer. Inga medie-original skickas in i WASM. UI kan aldrig mutera motorns projekt genom att ändra ett `inspect`-svar.

Backupformatet har åtta ASCII-byte `CSTUDIO1`, fyra byte big-endian headerlängd, UTF-8 JSON-header `{json, media:[{id,size,type}]}` och därefter respektive originalmedia. Längder valideras före Blob-slicing. Backupen kräver samtliga original; saknade original måste återlänkas först.

## 0.2 — tillägg

- `develop.set` tar valfri explicit `scope` (Scope-objekt eller `"project"`).
  `develop.reset` tar samma mål och tar bort dess justering. Utelämnat mål
  använder aktuell Workspace; UI fångar scope vid geststart.
- `clip.trim {id,start,end}` behåller ett halvöppet klipplokalt intervall och
  använder FilmCraft edit. Lager/justeringar räknas om, klippets source_in flyttas.
- `clip.move {id,index}` omordnar och behåller spelhuvudets aktiva klipp/lokala ruta.
- `clip.volume {id,volume}` accepterar 0–1 och når både preview och audioexport.
- `layer.text {text,scope?}` skapar Text `{content,size,color,position}`; storlek är
  relativ projektbredd, position normaliserad och färg RGBA 0–1.
- `layer.set` stödjer `text`, `name`, `mask: [x0,y0,x1,y1] | null`, `offset: [x,y]`
  samt tidigare synlighet/opacitet. `layer.scope {id,start,end}` ändrar lagrets tid.
- Nya `project_look`, `Clip.volume`, `Layer.text/mask/offset` har serde-standarder
  så första versionens filer fortfarande kan öppnas. Schemanummer är fortsatt 1.
- Dockning/namngivna layouter sparas endast i localStorage. Undo/redo bevarar
  aktuell Workspace och reparerar val som inte längre finns i innehållet.

## 0.3 — anslutna motorverktyg

- `inspect()` inkluderar `develop_controls` (92 stödda upstream ControlSpec) och
  `blend_modes` (27 faktiska PhotoCraft-lägen). UI visar 92 numeriska reglage:
  85 ytterligare motorreglage plus de sju tidigare; vitbalansen visas fortsatt
  som relativ temperatur/nyans. Absoluta `wb.temp`/`wb.tint` stöds via API.
- `develop.set.values` accepterar de exponerade numeriska kontroll-ID:erna,
  `treatment: "color" | "bw"` och `curve.master/red/green/blue` som 2–32
  `{x,y}`-punkter inom 0–1 med strikt stigande x. Alias för light.exposure,
  contrast, highlights, shadows och color.saturation normaliseras till de
  tidigare nycklarna; samma parameter lagras inte dubbelt.
- `view.set.pixel_selection` är en normaliserad rektangel eller null; detta
  är bildmarkering, separat från den tidsmässiga `selection`.
- `layer.stroke.stroke` stödjer `hardness`, `opacity`, `flow` (0–1),
  `pressure_size` och `selection` (rektangel eller null). Dessa sparas per
  operation och skickas direkt till PhotoCrafts paintmotor.
- `layer.set` stödjer `blend` (enum-ID från inspect), likformig `scale`
  (0.05–4) och `rotation` (-180–180 grader kring canvascentrum).
- `layer.duplicate {id}` kopierar innehåll och scope med nytt ID;
  `layer.move {id,index}` flyttar lagret i den globala lagerarrayen.
- Nya fält har serde-standarder. Tidigare versioners projekt, historik och
  backupformat fungerar fortsatt, med samma schemanummer 1.
