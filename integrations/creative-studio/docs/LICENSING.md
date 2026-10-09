# Licens- och varumärkesöversikt inför publicering

## Motorerna

PhotoCraft, LightCraft och FilmCrafts granskade versioner använder **MIT OR Apache-2.0**. Fullständiga licenser och respektive NOTICE finns i `public/licenses/{photocraft,lightcraft,filmcraft}/` och följer med statiska produktionsbyggnader. Creative Studios nya integrationskod har MIT-licens i rotens `LICENSE`.

De tre projekten har separat `docs/brand/LICENSE-brand.txt` och separat licens för appikonerna. ArtCraft-namn, wordmark och grafiska märken är inte öppna under kodlicensen. Varumärkesvillkoren kräver att modifierade/deriverade publicerade produkter tar bort märken och inte framställs som ArtCraft-produkter; användning utanför uttryckligt tillåtna sammanhang kräver rättighetshavarens tillstånd.

Creative Studio kopierar inga upstream-logotyper, appikoner, skärmbilder, översättningskataloger eller grafiska varumärkesfiler. Namnen PhotoCraft/LightCraft/FilmCraft förekommer som text för motorer/verktygslägen och attribution, inte som ArtCraft-produktmärke eller påstående om godkännande. Ny demoillustration och UI har skapats för detta projekt. Kontrollera produktnamn/domän och externa marknadsföringsmaterial separat före offentlig lansering.

## Webb- och Rust-beroenden

MP4Box.js använder BSD-3-Clause och webm-muxer MIT. Deras kompletta texter ingår i `public/licenses/`. Rust-låsfilens beroenden använder permissiva MIT/Apache/BSD/Zlib/Unlicense/0BSD-alternativ; ett paket kombinerar MIT/Apache med Unicode-3.0. `scripts/license-notices.py` samlar tillgängliga kompletta licenstexter och SPDX-information från exakt `cargo metadata --locked`, samt licenser för de två bundlade npm-runtimebiblioteken. Den genererade manifestfilen anger också utvecklingsberoenden som inte nödvändigtvis länkas in i WASM; inkluderingen är avsiktligt försiktig.

Kör insamlingen efter ändring av låsfiler och publicera `dist/licenses/` tillsammans med produkten. Kontrollera eventuella nya tillgångar (typsnitt, ikoner, exempelmedia) och nya codec-bibliotek innan de kopieras. Denna version använder systemtypsnitt och egen demo; ingen codec från ArtCraft Services återanvänds.

## ArtCraft Services och medieformat

ArtCraft Services har andra, begränsande fair-source-villkor och ingår inte i koden eller beroendegrafen. Inga AI-, konto- eller betalningsfunktioner har återanvänts.

Browserstöd för avkodning innebär inte automatiskt en bedömning av alla patent-/distributionsvillkor kring H.264/AAC. Den egna videoexporten använder VP9/Opus WebM. Om ytterligare codec-implementationer eller native-binära paket senare distribueras behövs en förnyad licens- och formatgranskning för den faktiska distributionen.

Den här filen dokumenterar teknikval och bevarade notices; den ersätter inte en eventuell juridisk granskning av kommersiell publicering.
