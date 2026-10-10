# Ett projekt, tre perspektiv · ADR-001

Status: implementerat, version 0.4, 2026-10-09.

## Auktoritativ modell

En `Studio` i en dedikerad Web Worker äger projektet, operationerna och den gemensamma historiken. `Project.timeline` är den enda sparade tidsstrukturen: en FilmCraft-sekvens med spår, klipp, effekter och övergångar. `Project.clips` är en härledd UI-projektion och serialiseras inte. Äldre projekt med sekventiella klipp migreras vid öppning.

Original, biblioteksmetadata, generatorer och nästlade sekvenser hör till samma projekt. UI-läge, panelplacering, zoom och markering skapar aldrig en ny projektinstans. Bara `Studio` sparar och hanterar undo/redo. Motorernas interna historik används högst som kortlivad arbetsinformation och lagras inte separat.

## Motoradaptrar

`src/native.rs` bygger PhotoCrafts dokumentvy från originalets lager, gemensamma lager och bildoperationer som gäller den aktuella tiden. Native operationer sparar kommandot, parametrarna, stabilt mållager, spatial markering, verktygsinställningar och explicit halvöppet tidsscope. Lager-ID:n i temporära dokument översätts till stabila alias. Urklipp som en paste-operation behöver lagras som lossless pcraft-data; det levande urklippet sparas inte i projektet.

LightCraft får samma komponerade canvas som PhotoCraft. Dess riktiga kommandon ändrar DevelopSettings; resultatet lagras i projektets justeringsoperationer. Lokala masker, spot removal, beskärning och förinställningar använder originalmotorn. Projektjusteringar sparar bara avvikande inställningar så att globala färgändringar bevarar lokala masker. Lokala masker/geometri kräver bildruta, intervall eller klipp. En återanvänd, härledd LightCraft-kommandovy tömmer katalog, källor och historik varje anrop: originalets desktop-downloader har en destructor som annars anropar en klocka som saknas i WASM. Vyn sparar aldrig ett eget projekt.

`src/film.rs` härleder en FilmCraft-vy från den auktoritativa sekvensen och projektbiblioteket. Native kommandon uppdaterar samma sekvens och metadata. Scopes följer källtid vid razor, trim, slip, hastighet och reverse. Nästlade klipp behåller sina inre scopes. Klippflytt eller panelbyte får inte vidga ett penseldrags omfattning.

## Gemensam rendering

1. Källplanen anger originalets tidspositioner för varje synligt klipp, inklusive övergångar och frame blending.
2. Originalet avkodas på begäran. PhotoCraft spelar upp de operationer som gäller den bildrutan.
3. LightCraft framkallar den komponerade bilden i flyttal efter konvertering från sRGB till linjär Rec.2020. Alfa följer samma geometri.
4. FilmCrafts riktiga `render_sequence` komponerar alla videospår, motion, opacity, effekter, övergångar, grafik och captions.

Canvas, filmrulle, PNG och videoexport använder denna renderingsväg. Native bildoperationer utvärderas vid projektupplösningen innan preview skalas ned. Canvasverktygens koordinater inverterar samma LightCraft-geometri och FilmCraft-motion som renderingen använder.

## Original och begränsade arbetsbuffertar

IndexedDB innehåller original som Blob och projekt/historik som JSON. Videobildrutor är tidsreferenser, inga exporterade PNG-filer. Browserkällor har ett LRU om fyra objekt; native avkodarkällor har ett LRU om två objekt. `src/sources.rs` ger originalmotorerna range-läsning genom `Blob.slice` i arbetaren. Originalfilm läses aldrig som en hel ArrayBuffer. PSD/pcraft och andra stillbilder får läsas i sin helhet inom 64 MB/16 MP-importgränsen. Återlänkning invaliderar avkodarcachen.

Projekt/bildbuffert har högst 16 777 216 pixlar. Operationer, punkter, spår, samtidiga källor och tidslinjelängd valideras. Studio behåller högst 30 innehållssteg och 15 MB serialiserad historik; aktuella projektoperationer har också en 15 MB-budget. Originalmedia dupliceras inte i historiken.

## Ljud, uppspelning och export

WAV läses i sampleintervall; MP3/FLAC/AIFF använder Symphonia med en range-adapter, samma decoderfamilj som FilmCraft. Containerljud använder WebCodecs eller native avkodare. Bounded PCM-fönster resamplas och skickas till FilmCrafts riktiga audio-DSP/mixer med spår, gain, effekter och mute. Uppspelning följer AudioContext-klockan.

Videoexport samplar varje projektruta med rationella tidsstämplar. WebM använder VP9/Opus. MP4 använder browserns H.264 när tillgängligt och annars FilmCrafts egen strömmande H.264-kodare, med AAC eller Opus-ljud. Kodarköer och filskrivningar dräneras under arbetet. Nedladdning i minnet begränsas till två minuter; sökbar direktlagring stöder längre export och avbrott. Exportkodaren är en tillfällig resurs utan projektmodell.

8-bitars sRGB är fortfarande in/ut-kontraktet för den gemensamma canvasen. CPU-preview garanterar inte att varje ruta hinner visas under uppspelning. Export renderar varje ruta. HDR/ICC/16-bitars trohet och lång högupplöst export behöver fortsatt arbete.

## Fem arbetsytor, en modell

`Project.graphic_layers` lagrar VectorCraft-/DesignCraft-kommandon och ett klippbundet `[start, end)`-intervall. Originalmotorn härleder ett redigerbart dokument genom journalåterspelning; dess CPU-renderare skapar ett transparent lager i samma PhotoCraft-komposition. LightCraft framkallar sedan den gemensamma bildrutan och FilmCraft renderar sekvensen. Ingen VectorCraft-/DesignCraft-session eller separat undo-stack sparas. Grafikrastrar cachas med exakta journalnycklar i högst 32 MB, aldrig per videobildruta.

Grafikens tidsomfattning måste matcha verktygets val för objektredigering. Trimning, duration och native razor ommappar scopes och PhotoCraft-alias; grafikband på tidslinjen kan ändra intervallet explicit. Fem lägen väljer egna verktygsrader och dockflikar. Layoutmigration behåller tidigare fyra-panel-konfigurationer och kompletterar med de nya panelerna.
