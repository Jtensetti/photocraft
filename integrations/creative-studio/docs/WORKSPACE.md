# Creative Studio 0.2 — arbetsytan

Canvasen sitter kvar i mitten. Verktygsväxling öppnar en panel och lämnar projekt,
spelhuvud, markering och historik intakta. PhotoCraft och LightCraft kan därför
synas samtidigt. Panelhuvudet kan dras till den andra dockan; placeringsmenyn
stödjer också vänster, höger och en flyttbar frikopplad panel. Dockor kan visa
flikar eller staplade paneler. Paneler kan minimeras, döljas och återöppnas.
Sidornas bredd och filmrullens höjd kan ändras genom att dra i respektive kant.
Namngivna arbetsytor inkluderar dockning, panelbredder, sidornas synlighet samt
filmrullens höjd och synlighet. De sparas separat i localStorage, utanför innehållshistoriken.

Filmrullen zoomar kontinuerligt från projektöversikt till en miniatyr per bildruta.
Klick väljer en ruta, Shift-klick förlänger intervallet från senaste klick och drag
markerar ett sammanhängande intervall **inom samma klipp**. Dubbelklick väljer
klippomfattning. I/O och piltangenter fungerar också. Sekvensens globala tid och
klippets lokala bildrutenummer visas var för sig. Miniatyrer renderas från
originalet vid behov, inklusive samma Rust-operationer som canvas/export. Bara
synliga miniatyrer skapas; cachen är högst 128 bilder och två avkodare, separat
från uppspelning. Vid mycket långa tidslinjer flyttas det synliga fönstret kring
spelhuvudet för att undvika webbläsarens maximala elementbredd.

**Spår** visar den sekventiella mediekanalen, originalljud och ett tidsband per
pensel-/textlager. Dra bandets ändhandtag för att ändra lagrets tidsomfattning.
Detta är ännu inte flera oberoende videospår eller en ljudmixer.

Redigeringens omfattning visas ovanför canvasen. Reglage och pensel fångar sitt
scope när gesten börjar; en fördröjd uppdatering får aldrig byta mål efter en ny
markering. Projektomfattning stöds för fotografiska justeringar. Explicit valda
projektparametrar appliceras efter lokala parametrar; återställning tar bort
justeringen inom vald omfattning. Lager kräver bildruta, intervall eller klipp.

Text renderas med PhotoCrafts textmotor och bundlad Inter, utan HTML-text ovanpå
canvasen. Text, pensel, rektangulär lagermask, opacitet, synlighet och förflyttning
sparas i samma lageroperationer och följer med i PNG och video. Textverktyget
skapar lager i vald omfattning; det valda lagrets start/slut kan sedan ändras i
panel eller tidsband. Förflyttning är X/Y; fri rotation/skala och penselmasker
återstår.

FilmCraft-trimning skapar en tillfällig vy för `filmcraft-edit::trim`. Ingen
FilmCraft Session eller separat sparad FilmCraft-fil skapas. Gemensamma scopes
klipps till det behållna intervallet och räknas om mot samma källbildrutor.
Omordning behåller klipp-ID, lokal bildruta och tillhörande operationer. Ljudets
volym gäller både preview och export.

Undo/redo flyttar bara innehåll mellan snapshots. Aktuell arbetsyta bevaras;
spelhuvud, markering och lagerval repareras om återställd tidsstruktur gör dem
ogiltiga. Automatiska återställningspunkter sparas efter innehållsändringar, utan
att märka manuellt arbete som sparat. Startvyn och projektlistan erbjuder
återställning. Återställning av backup-media använder en enda IndexedDB-
transaktion så att ett lagringsfel inte lämnar ett halvåterställt mediebibliotek.

WebM kan laddas ned i minnet upp till 120 sekunder. **Spara video direkt till fil**
använder File System Access i Chrome/Edge och en sökbar StreamTarget. Utdata skrivs
successivt med 1 MiB muxerchunkar och dränerade köer; ljud och bild kodas
interfolierat. En avbruten/felaktig direktlagring anropar `abort()` på filströmmen.
Original läses fortfarande från Blob-delar. Testet avkodar en 121-sekunders
export via en verklig OPFS-filström; en full 30-minuters kvalitets- och
minnesprofil är fortfarande framtida validering.
