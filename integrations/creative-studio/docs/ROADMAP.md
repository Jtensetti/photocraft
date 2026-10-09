# Faktisk funktionsstatus och fortsättning

## Levererat i 0.3

| Område | Funktion |
|---|---|
| Projekt | En Rust Studio, canvas, tidsstruktur, historik, save/open och backup |
| Filmrulle | Riktiga tidsbaserade miniatyrer, bildrutezoom, klick, Shift-intervall, dragmarkering och I/O |
| Scope | Bildruta, klippintervall, klipp och fotografisk projektjustering; gestens mål fångas uttryckligen |
| Utformning | Tät charcoal-arbetsyta med originalens Pro-mönster, Lucide-verktygsrad, fungerande menyer, kontextinställningar, dokumentflik, zoom och handverktyg |
| Arbetsyta | Flyttbara dockor, flikar/stapling, minimering, frikopplade paneler, bredder, höjd och namngivna layouter |
| PhotoCraft | Faktisk paint/compose/text, penselhårdhet/opacitet/flöde/tryck, sudd, rektangulär bildmarkering/lagermask, 27 blandningslägen, X/Y, centrerad skala/rotation, duplicering/ordning och tidsomfattning |
| LightCraft | Faktisk pipeline, RGB-punktkurvor, 92 numeriska UI-reglage för ljus/färg/HSL/svartvit mix/gradering/effekter/vinjett/korn/detalj/kalibrering, histogram och gemensamma icke-destruktiva justeringsoperationer |
| FilmCraft | Faktisk time/project/edit-adapter för trimning; split, omordning, stillbildslängd, mute och klippvolym |
| Spårvy | Sekventiella klipp, originalljud och separata tidsband för pensel-/textlager med ändhandtag |
| Export | Bearbetad PNG, exakt VP9/Opus WebM, interfolierad kodning, avbrott och direktlagring för längre sekvenser |
| Lagring | IndexedDB, portabel backup, relink, automatisk återställningspunkt och atomisk backup-medieåterställning |
| Historia | Undo/redo bevarar arbetsytan och reparerar ogiltig markering/lagerval efter tidsändringar |

20 Rust-tester kontrollerar verkliga pixlar, scope, text, mask, kurvor/HSL,
blandningslägen, transformering, parameteralias, trimning, omordning, atomiska
fel, äldre standardfält och historikbudget. Webbläsartester avkodar
exporter med ffmpeg och kontrollerar video, ljud, intervall samt en 121-sekunders
OPFS-export. Se tests och [arbetsytans kontrakt](WORKSPACE.md).

0.3 bygger vidare på samma sparformat och bakåtkompatibla standardfält. Den
exponerar ett större urval av de befintliga motorerna; menyerna listar anslutna
operationer. Detta är fortfarande inte full Photoshop/Lightroom/Premiere-paritet.

## Kvarvarande gränser

- En sekventiell mediekanal. Lagerbanden är riktiga overlay-spår, men oberoende
  video-/ljudspår, mixning, övergångar och den fullständiga FilmCraft-renderaren
  återstår. Spårvyn utger sig inte för att ha dessa funktioner.
- CFR-MP4 med samma projekt-fps. VP9/Opus är avkodningstestat; H.264/AAC måste
  verifieras i vanlig Chrome/Edge. VFR, separata ljudfiler, 44.1 kHz resampling,
  komplexa MP4-edit-listor och fler exportcontainrar återstår.
- Direktlagring är testad över den tidigare tvåminutersgränsen vid liten upplösning.
  En 30-minuters export i hög upplösning behöver separat minnes-/prestandaprofil.
  MP4-indexet och WebCodecs interna codec-buffertar har fortfarande egna kostnader.
- Mask och bildmarkering är rektangulära och ligger i canvasens koordinater;
  de följer ännu inte med när lagret transformeras. Skala är likformig kring
  canvascentrum; fri perspektivtransform, lasso, penselmask och lokala
  LightCraft-masker återstår.
- Text använder en bundlad font och begränsad enkel stil. Typografiska
  run-stilar, fontval och objektbaserade transformhandtag återstår.
- Canvas in/ut är 8-bitars sRGB. HDR, ICC/RAW/16-bitars trohet, GPU-rendering,
  keyframes och tracking återstår.
- Automatisk återställningspunkt ersätter inte en portabel backup. En arbetare
  som kraschar måste fortfarande laddas om; strukturellt delad historik,
  innehållshash vid relink och fullständigt migrationsregister återstår.
- Intervall sträcker sig inom ett klipp. Flerval av åtskilda rutor eller över flera
  klipp kräver ett scope-set i nästa projektversion.

## Nästa accepterbara steg

1. Härled FilmCrafts flerspårssekvens och SourceProvider från samma projekt;
   migrera scopes mot källtid innan oberoende klippplacering/övergångar införs.
   Avkoda exporten och verifiera överlapp, ljudplacering och alla intervallgränser.
2. Exakt sample-indexerad VideoDecoder/VFR-adapter och fler format. Verifiera
   23.976/29.97, H.264/AAC, resampling och 30 min under explicit minnesbudget.
3. Anslut PhotoCrafts fria transform/vector-mask och LightCrafts lokala masker.
   Koppla lager och masker till samma transform; lägg till objektets egna
   transformhandtag och färgreferenser för preview/export.
4. Journal och worker-återstart, innehållshash, schema-migreringar och kvottester.

Ett eget `Jtensetti/creative-studio`-repository kan inte skapas genom GitHub-
anslutningen i denna session. Mappen är självständig med egen Cargo-workspace och
CI; publiceringskällan och webbdemon är separata från PhotoCrafts desktopapp.
