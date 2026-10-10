# Verifierad funktion och kvarvarande gränser

## Version 0.5

| Område | Implementerat |
|---|---|
| Gemensamt projekt | En Rust Studio, auktoritativ FilmCraft-sekvens, tidsmedvetna operationer, canvas och gemensam historik |
| Filmrulle | Verkliga tidsminiatyrer, zoom till bildrutor, klick, Shift-intervall, dragmarkering, I/O och klipp-/lagerband |
| Arbetsyta | Flyttbara paneler, dockor, flikar/stapling, frikoppling, minimering, dimensioner och namngivna layouter |
| Fem gränssnitt | Egen verktygsrad för Photo/Light/Film/Vector/Design, flyouts för retusch/markering/paint, aktiv dockflik med möjlighet att kombinera paneler |
| VectorCraft | Originalmotor och renderare, former/banor/text, objektval, fyllning/linjer/opacitet, transformation, gruppering och duplikat på tidsmedvetna grafiklager |
| DesignCraft | Originalmotor, typsättning och renderare, ramar/text/linjer, objektval, fyllning, transformation och textramar med kolumner/gutter/inset |
| Verktyg | Originalens verkliga kommandoregister, sökning/kategorier/parameterformulär och tydliga plattformsbegränsningar |
| PhotoCraft | Native dokument/lager, filter, lasso/trollstav, retusch, kloning/lagning, gradient/fill, former/vektorer, masker, fria transformationer, stilar, kanaler, urklipp och penselförinställningar |
| LightCraft | Full pipeline, kurvor/HSL/gradering/detalj, lokala masker med alla justeringar, brush/linear/radial, spot removal, beskärning/geometri, auto och förinställningar |
| FilmCraft | Full native renderare, flera spår, dragflytt/trim/razor, ripple/slip/slide, speed/reverse, övergångar, effektparametrar/keyframes, native grafik och nästlade sekvenser |
| Ljud | Flera ljudspår, gain/mute/effekter, faktisk native DSP-mix, resampling, fristående WAV/MP3/FLAC/AIFF och stödda containerljud |
| Original | Browserformat samt PhotoCrafts pcraft/PSD/PSB/TIFF/RAW-import och FilmCrafts range-/GOP-avkodare; inga fysiska videobildrutefiler |
| Export | Samma renderade canvas i PNG, WebM och MP4; strömmande H.264-fallback från FilmCraft; avbrott och direktfil för längre export |
| Spara | IndexedDB, portabel backup med original, relink, automatisk återställningspunkt, atomisk validering och gemensam undo/redo |

Inventeringen omfattar 833 PhotoCraft-, 264 LightCraft- och 675 FilmCraft-kommandospecifikationer. Antalet inkluderar desktopfunktioner och administrativa kommandon. Det är inte ett påstående om att 1 772 funktioner har full browserparitet eller har testats individuellt.

Rust-tester kontrollerar faktiska pixlar, scope, masker, urklipp, kurvor, transformationskoordinater, spår, övergångar, nästling, speed/reverse, audio-DSP, atomiska fel och historikbudget. Playwright verifierar UI, sparning/återöppning, lageroriginal, specialformat, ljud och avkodade exporter. En separat 121-sekunders OPFS-export kontrollerar strömning och avbrott. Produktionsbygget testas från en undermapp.

## Gränser som ännu finns

- Desktopfiler, hårdvaruenheter, plugins, modellnedladdning och AI-modeller är inte webbanpassade. Kommandon som kräver dem markeras eller ger ett konkret adapterfel. Ingen extern AI-/konto-/betalningstjänst ingår.
- Canvasen har 8-bitars sRGB in/ut. RAW kan importeras av originalmotorn, men HDR, ICC och 16-bitars trohet bevaras ännu inte genom hela kedjan.
- UI visar native filter och många avancerade funktioner i originalens parameterformulär. Interaktiva transformhandtag, avancerade biblioteksvyer och full meny-/dialogparitet med desktoporiginalen behöver fortsatt utformning.
- Intervallet gäller ett klipp. Åtskilda rutor och flerval över klipp kräver ett scope-set. Verktyg som behöver ett större mållager ger fel om dess tidsomfattning inte täcker valet.
- Tillgängligt codec-stöd och vissa komplexa container-/edit-listor varierar. De automatiserade formatkontrollerna täcker konkreta fixtures och garanterar inte varje kameraformat eller skadad fil.
- Lång export är verifierad vid liten upplösning. En halvtimmes 4K-export och stora lagerprojekt behöver separat minnes-/prestandaprofil. Originalavkodare, originalens cache och codec-index har egna kostnader.
- Native operationer måste kunna spelas upp på sitt mål; vissa verktyg för externa presets, flera dokument eller separata bibliotek behöver en ytterligare browseradapter. Full desktopparitet är inte färdig.
- Media är lokal per origin. Portabel backup behövs för flytt mellan domäner/enheter. Filhash vid relink, automatisk återstart av en kraschad worker och ett fullständigt migrationsregister återstår.

- VectorCraft och DesignCraft är nya integrationer. Särskilda fil-/biblioteksadaptrar, alla interaktiva originalverktyg och DesignCrafts flersidespresentation är ännu inte färdiga. Parametriska originalkommandon finns i verktygssökningen med JSON-komplettering; detta motsvarar inte full dialog- eller gestparitet.

Nästa arbete är därför browseranpassning och UX för de återstående originalfunktionerna, färgtrohet och storprojektsprofilering. Den gemensamma modellen och originalmotorerna ska fortsatt vara grunden.
