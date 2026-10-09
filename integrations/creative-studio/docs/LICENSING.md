# Licens- och varumärkesöversikt inför publicering

## Motorerna

PhotoCraft, LightCraft och FilmCrafts granskade versioner använder **MIT OR Apache-2.0**. Fullständiga licenser och respektive NOTICE finns i `public/licenses/{photocraft,lightcraft,filmcraft}/` och följer med statiska produktionsbyggnader. Creative Studios nya integrationskod har MIT-licens i rotens `LICENSE`.

De tre projekten har separat `docs/brand/LICENSE-brand.txt` och separat licens för appikonerna. ArtCraft-namn, wordmark och grafiska märken är inte öppna under kodlicensen. Varumärkesvillkoren kräver att modifierade/deriverade publicerade produkter tar bort märken och inte framställs som ArtCraft-produkter; användning utanför uttryckligt tillåtna sammanhang kräver rättighetshavarens tillstånd.

Creative Studio kopierar inga upstream-logotyper, appikoner, skärmbilder, översättningskataloger eller grafiska varumärkesfiler. Namnen PhotoCraft/LightCraft/FilmCraft förekommer som text för motorer/verktygslägen och attribution, inte som ArtCraft-produktmärke eller påstående om godkännande. Ny demoillustration och UI har skapats för detta projekt. Kontrollera produktnamn/domän och externa marknadsföringsmaterial separat före offentlig lansering.

## Webb- och Rust-beroenden

MP4Box.js använder BSD-3-Clause; webm-muxer och mp4-muxer använder MIT. Symphonias ljudavkodare använder MPL-2.0 och är återanvända utan källändringar. Deras kompletta texter ingår i `public/licenses/`. Den låsta Rust-grafen innehåller också MIT/Apache/BSD/Zlib/ISC/Unlicense-alternativ, Unicode-, IJG-, NCSA- och CDLA-texter. `scripts/license-notices.py` samlar tillgängliga kompletta licenstexter och SPDX-information från exakt `cargo metadata --locked`, samt licenser för de tre bundlade npm-runtimebiblioteken. Den genererade manifestfilen anger även paket som inte nödvändigtvis länkas in i WASM.

Kör insamlingen efter ändring av låsfiler och publicera `dist/licenses/` tillsammans med produkten. Denna version använder systemtypsnitt i UI, PhotoCrafts bundlade texttypsnitt, Lucide-verktygsikoner och en public-domain-demobild; se `ATTRIBUTION.md`. Codec-biblioteken kommer från de låsta öppna originalmotorerna, inte ArtCraft Services.

## ArtCraft Services och medieformat

ArtCraft Services har andra, begränsande fair-source-villkor och ingår inte i koden eller beroendegrafen. Inga AI-, konto- eller betalningsfunktioner har återanvänts.

Browserstöd för avkodning innebär inte automatiskt en bedömning av alla patent-/distributionsvillkor kring H.264/AAC. Export stöder VP9/Opus WebM och H.264 MP4; MP4 använder browserkodare eller FilmCrafts egen H.264-kodare med originalets MIT/Apache-notices. Ljud är AAC eller Opus beroende på browserstöd. Filen dokumenterar kodlicenser och implementation, inte en separat codec-patentbedömning.

Den här filen dokumenterar teknikval och bevarade notices; den ersätter inte en eventuell juridisk granskning av kommersiell publicering.
