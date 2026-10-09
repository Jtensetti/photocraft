# Faktisk funktionsstatus och fortsättning

## Levererat i 0.2

| Område | Funktion |
|---|---|
| Projekt | En Rust Studio, canvas, tidsstruktur, historik, save/open och backup |
| Filmrulle | Riktiga tidsbaserade miniatyrer, bildrutezoom, klick, Shift-intervall, dragmarkering och I/O |
| Scope | Bildruta, klippintervall, klipp och fotografisk projektjustering; gestens mål fångas uttryckligen |
| Arbetsyta | Flyttbara dockor, flikar/stapling, minimering, frikopplade paneler, bredder, höjd och namngivna layouter |
| PhotoCraft | Faktisk paint/compose/text, lager, sudd, opacitet, rektangulär mask, X/Y-förflyttning och tidsomfattning |
| LightCraft | Faktisk pipeline och sju ljus/färgreglage, gemensamma icke-destruktiva justeringsoperationer |
| FilmCraft | Faktisk time/project/edit-adapter för trimning; split, omordning, stillbildslängd, mute och klippvolym |
| Spårvy | Sekventiella klipp, originalljud och separata tidsband för pensel-/textlager med ändhandtag |
| Export | Bearbetad PNG, exakt VP9/Opus WebM, interfolierad kodning, avbrott och direktlagring för längre sekvenser |
| Lagring | IndexedDB, portabel backup, relink, automatisk återställningspunkt och atomisk backup-medieåterställning |
| Historia | Undo/redo bevarar arbetsytan och reparerar ogiltig markering/lagerval efter tidsändringar |

Rust-tester kontrollerar verkliga pixlar, scope, text, mask, trimning, omordning,
atomiska fel, äldre standardfält och historikbudget. Webbläsartester avkodar
exporter med ffmpeg och kontrollerar video, ljud, intervall samt en 121-sekunders
OPFS-export. Se tests och [arbetsytans kontrakt](WORKSPACE.md).

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
- Mask är rektangulär; förflyttning är X/Y. Fri transform, vektormarkering,
  penselmask, kurvor och lokala LightCraft-masker återstår.
- Text använder en bundlad font och begränsad enkel stil. Typografiska
  run-stilar, fontval och textposition genom drag på canvasen återstår.
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
3. Återanvänd PhotoCrafts transform/vector-mask och LightCrafts kurv/mask-API:er;
   jämför faktisk canvas och export mot samma operationer och färgreferenser.
4. Journal och worker-återstart, innehållshash, schema-migreringar och kvottester.

Ett eget `Jtensetti/creative-studio`-repository kan inte skapas genom GitHub-
anslutningen i denna session. Mappen är självständig med egen Cargo-workspace och
CI; publiceringskällan och webbdemon är separata från PhotoCrafts desktopapp.
