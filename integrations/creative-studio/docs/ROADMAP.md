# Faktisk funktionsstatus och fortsättning

## Levererat och automatiskt verifierat

| Funktion | Status |
|---|---|
| Ett projekt, canvas, historik och verktygsswitch | Implementerat; Rust- och webbläsartest |
| Tom bild, import PNG/JPEG/WebP och CFR-MP4 | Implementerat; MP4 testad i webbläsare |
| Lager, pensel, sudd, opacitet, synlighet | Implementerat med PhotoCraft; verkliga pixlar testade |
| Sju fotografiska reglage för bildruta/intervall/klipp | Implementerat med LightCraft; intervallgränser, WB och pixelresultat testade |
| 100 bildrutor delar en justeringsoperation | Rust-test för `[200,300)` |
| Enbart bildruta 137 målas | Rust-test av ruta 136/137/138, undo/redo och återöppning |
| Filmdelning och bevarat originaloffset | Implementerat; modelltest och ljudtest av exporterad film |
| Lokal save/open med historik, portabel backup | Implementerat; återställning efter raderat original i IndexedDB testad |
| PNG och deterministisk VP9/Opus WebM-export | Implementerat; faktisk fil avkodad och kontrollerad |
| Paneler/tidslinje kan döljas, höjd, fokus, sparad layout | Implementerat; dold tidslinje/lägesbyte testat |

## Delvis levererat eller fortfarande begränsat

| Område | Gräns |
|---|---|
| FilmCraft-integration | Faktisk `filmcraft-time`; full projekt-/render-/exportmotor återstår |
| Videoformat | CFR-MP4 med samma projekt-fps; testad VP9/Opus. H.264/AAC kräver egen verifiering på vanlig Chrome/Edge. |
| Ljud | Browser-preview och 48 kHz mono/stereo MP4-export; ett ljudspår, muting, enkel edit-lista. Inga separata ljudassets/mixer. |
| Export | WebM upp till två minuter; ingen H.264-MP4-export, längre streamad export eller batchexport. |
| Tidslinje | Ett sekventiellt spår med underliggande originalljud; split, bildrutenavigation och stillbildsvaraktighet. Ingen omordning eller vanlig video-in/out-trim i UI. |
| Layout | Dölj/visa, höjd, maximera och spara; ingen flyttbar/dockbar panelmotor. |
| Färg | Flyttalsbearbetning internt; 8-bitars sRGB canvas in/ut. Inte färgkritisk HDR/ICC/RAW-leverans. |
| Återlänkning | Storlek/typ/dimensioner jämförs, ingen innehållshash ännu. |
| Resurser | Bounded sources/preview/export och metadatahistoria; Historiken har en 15 MB-budget. OPFS och strukturellt delad historik behövs för större projekt. |

## Nästa leveranser, med acceptanskriterier

1. **Media och längre export:** WebCodecs VideoDecoder med exakt sample-index/CFR/VFR-adapter, OPFS/streaming-muxer, progress för ljudindexering, kvot- och avbrottstester. Export av 30 min får inte hålla original eller full kodad utdata i minnet. Verifiera 23.976/29.97 fps, H.264/AAC, 44.1 kHz resampling och flera MP4-edit-listor mot avkodad referens.
2. **FilmCraft-adapter:** härledd `filmcraft-project`-renderingsvy och `SourceProvider`, tydligt ägande av gemensam kommandotransaktion. Flera video-/ljudspår, riktig trimning/omordning, övergångar. Testa källtid, överlapp, ljudplacering och scope-migrering efter varje tidslinjeändring. Behåll en auktoritativ historik.
3. **PhotoCraft-objekt och masker:** återanvänd text/vector/maskbibliotek bakom nya gemensamma operationstyper. Transformationer och lagergrupper. Testa intervallbaserad text, mask och faktisk PNG/videoexport, inklusive transparenta bilder.
4. **LightCraft och färg:** kurvor, lokala masker, presets och synkronisering, RAW/16-bitars/ICC/linjär HDR-kedja. Behåll upstream-rendering; testa mot numeriska och visuella referenser utan att klippa högdagrar i canvasövergången.
5. **Arbetsyta och animation:** dockning, namngivna layouter, keyframes med uttrycklig interpolation. Tracking som separat, opt-in producer av tidsbaserade parametrar. Testa att tidslinjedöljning och lägesbyte aldrig ändrar renderat innehåll.
6. **Projektformat och driftsäkerhet:** migrationsregister, innehållshash för relink, autosave/journal och worker-återstart, historik med strukturell delning, import som atomisk lagringstransaktion. Testa full disk, stängning mitt i save/backup, saknad media och korrupt fil.

Avancerade masker, text, retuschverktyg, keyframes, tracking, AI-generering, marknadsplats och kontofunktioner visas inte som fungerande verktyg i första UI. AI, marknadsplats och användarkonton ingår inte i projektets plan.
