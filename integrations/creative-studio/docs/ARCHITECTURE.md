# Arkitekturbeslut · ADR-001

Status: implementerat för första vertikala versionen, 2026-10-09.

## Ett auktoritativt projekt

En `Studio` i en dedikerad Web Worker äger det versionshanterade Rust-projektet, alla redigeringsoperationer och gemensam undo/redo. Webbsidan skickar kommandon och visar `inspect()`-svaret. Den äger inte en andra redigerbar dokumentmodell. Verktygsläget är ett arbetsytefält och påverkar endast synliga paneler.

En bildruta är resultatet av att evaluera projektet vid en global bildruteidentifierare. Klipp refererar till ett original och dess `source_in`; stillbilders varaktighet lagras som ett heltal, inte som upprepade bilder. Ett scoped pensellager eller en framkallningsoperation är ett objekt med ett halvöppet intervall i klippets lokala tid. Vid delning delas korsande operationers intervall och det högra klippets originaloffset flyttas fram; originalmedia dupliceras inte.

## Återanvänd motorbibliotek, inte tre Session-objekt

PhotoCrafts `Session` äger dokument, LightCrafts `Session` en katalog och FilmCrafts `Session` ett eget projekt. Att anropa deras kommandoregister direkt skulle kräva tre synkroniserade persistenta modeller. Första versionen återanvänder i stället lägre, fristående funktioner:

- `photocraft-paint::render_stroke` spelar upp penseloperationer och `photocraft-compose::render` komponerar lager.
- `lightcraft-pipeline::render` använder `DevelopSettings` med projektets aktiva parametrar. sRGB avkodas och konverteras till linjär Rec.2020 innan framkallningen. Vitbalans använder uttryckligen Custom-läge. Originalets alfa behålls.
- `filmcraft-time::{FrameRate, Tick}` håller rationell bildfrekvens och exakt projekttid.

PhotoCrafts `Document` som byggs vid rendering är en temporär kompositionsvy, utan egen Session, historik eller sparformat. Den försvinner efter bildrutan. Originalets avkodade pixlar, LightCraft-resultatet och pensellagren skickas genom samma funktion för förhandsvisning, PNG och videobildrutor.

FilmCrafts fullständiga render- och exportbibliotek är **inte** anslutna i denna version. Projekt/edit används genom den härledda trimningsadaptern. Sekventiell klipptid, delning och muting finns i det lilla gemensamma integrationslagret. Att koppla in dessa bibliotek kräver en adapter från den gemensamma modellen till en härledd FilmCraft-renderingsvy och en `SourceProvider`. Adaptern får inte introducera ett separat sparat projekt eller dubbla undo-stackar.

## Webbmedier och resursgränser

HTMLVideoElement och ImageBitmap avkodar original på begäran. Ett MP4-index läses via högst 1 MB stora Blob-delar, med hopp över mediadelen enligt MP4Box. Ingen hel video läses som ArrayBuffer. Ett LRU behåller upp till fyra källobjekt; video pausas och objekt-URL:er återkallas vid byte/eviction. Miniatyrer är 160×90.

IndexedDB innehåller original som Blob-objekt och sparade projekt som JSON. Backupen består av en längdprefixad JSON-header och Blob-delar av originalen. Återställning validerar först projektet i Rust. Det finns ingen nätverksväg för uppladdning av användarens media.

Förhandsvisning begränsas till 960×640. Projekt och bildrutebuffert begränsas till 16 777 216 pixlar, med högst 16 384 pixlar per sida. Projektgränser finns också för objekt, tidslinjelängd och penselpunkter. Historiken behåller högst 30 innehållskommandon och 15 MB serialiserade metadata-/operationssnapshots, aldrig kopior av videofiler eller avkodade bildrutor. Äldre snapshots tas bort när budgeten nås. Aktuella projektoperationer begränsas också till 15 MB, så sparformatet alltid kan öppnas inom 32 MB-gränsen. Strukturell delning är nästa optimering för stora penselprojekt.

## Tidsstämplar och ljud

Projektets bildrutetid kommer från FilmCraft. Exportens mikrosekunder räknas med heltal och rationell bildfrekvens för varje bildruta; den ackumulerar inte en flyttalsklocka. WebCodecs kodar varje bearbetad bildruta som VP9. WebM-muxern skriver seekbar utdata med tidsstämplar och varaktighet. Realtidsinspelning används inte för export.

AudioDecoder läser komprimerade ljudpaket via Blob-delar, normaliserar MP4-edit-listans tidsförskjutning, beskär vid klippets originaloffset och placerar samples på tidslinjens 48 kHz-klocka. Mutade klipp och stillbilder fylls med tystnad. AudioEncoder kodar en sammanhängande Opus-ström; den flushas en gång vid slutet för att undvika upprepad codec-padding. Decoder/encoder-köer töms med backpressure. Video/audio-dataobjekt stängs efter användning.

Förhandsvisning följer videons medieklocka så att långsam CPU-rendering inte driver spelhuvudet framför ljudet. Den ger inte en garanti om att varje bildruta hinner visas under uppspelning. Exporten gör det. Nedladdning i minnet begränsas till två minuter. Direktlagring till en sökbar filström stödjer längre sekvenser med begränsade muxer- och codec-köer.

## Utökning utan att byta grundprincip

Text, rektangulär mask och centrerad skala/rotation finns nu i gemensamma lageroperationer. Schemat behöver senare former, fria masker, keyframes och tracking; dessa finns ännu inte som tomma låtsasobjekt. De ska använda samma scope och tydliga enheter/tidsbas. Keyframes samplas vid renderingtiden. Tracking ska producera tidsbaserade parametrar; penseldrag får aldrig automatiskt tolkas som rörelsespårning.

När fler spår och övergångar införs ska implicit sekventiell klipptid ersättas genom en explicit schemamigrering. GPU-, HDR-/ICC- och RAW-adaptrar ska gå bakom samma renderingstjänst. Inga ändringar av uppströmsprogrammens fungerande modeller behövs för den nuvarande versionen.

## 0.2-adapter och UI

`web/layout.js` äger endast panelplacering. `web/filmstrip.js` projicerar samma
klipp och operationer, med synliga tidsminiatyrer och separat begränsad avkodarcache.
`src/film.rs` bygger en tillfällig FilmCraft-editvy för trimning; inget extra
projekt sparas eller äger historik. Text och masker renderas av PhotoCraft.
Utdata från `web/export.js` kan gå till en sökbar filström, med interfolierad
ljud-/videokodning och dränerade 1 MiB-utdataköer.

## 0.3 — Pro-gränssnitt utan extra projektmodell

`web/pro-ui.js` projicerar menyer, verktygsval och canvaszoom.
`web/develop-ui.js` bygger LightCrafts reglage från inspekterade motorgränser
och visar histogram/kurvor. Utökade fotografiska parametrar ligger i samma
sparsamma justeringsoperationer som tidigare. UI håller ingen egen framkallningsfil.
PhotoCrafts BrushSettings, Affine och BlendMode används i den befintliga
renderingsvägen. Både temporala och spatiala penselmål sparas explicit per drag.
