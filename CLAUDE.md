# Glance – dashboard-app voor Mira-glasses

Gebruiker is Belg en spreekt Nederlands; antwoord in het Nederlands. UI-teksten op bril en telefoon zijn Nederlands (Vlaams-vriendelijk).

## Werkwijze
- Volg altijd de skill `mira-wasm-app` (`.claude/skills/mira-wasm-app/SKILL.md`): Rust/WASM op de bril, HTML/JS-companion op de telefoon, één HTML-bestand opleveren.
- Toolchain: `rustup target add wasm32-unknown-unknown` (Python 3 en Node.js zijn nodig voor de validator).
- SDK-bronnen (`Shared/`, `SDK-API.md`, `SDK-ABI.json`, `CompanionExample/`) zijn uit de skill geëxtraheerd; niet zelf herschrijven.

## Bouwen en valideren
```sh
python3 Glance/package.py Glance.html
python3 Glance/validate.py Glance.html
```
Lever alleen `Glance.html` op. Installeren: Apps → Create an app → Upload → Check code → Add → Install → Open.
Bijwerken: Glance-kaart lang indrukken → Edit App → Upload → Save → Update → Open.

## Structuur
- `Glance/glance.rs` – bril-app (app-naam "Glance"). Pagina's: overzicht, agenda, weer, nieuws, markten, to-do. Swipe = pagina, tik = vernieuwen (op nieuws: volgende koppen).
- `Glance/companion.html` – telefoon: haalt Open-Meteo, iCal-links, RSS (standaard HLN, met reserve-URL), Yahoo Finance (standaard ^BFX, ABI.BR, BTC-EUR) op.
- Protocol telefoon → bril: `setSnapshotForApp({type:'dash', d:<tekst>})`, regels `TAG|veld|veld`:
  `P` paginavolgorde, `W` weer, `H` per uur, `D` per dag, `E` afspraak, `N` nieuws, `S` koers, `T` taak, `U` bijgewerkt. JSON ≤ 3400 bytes.
- Bril → telefoon: `{"type":"sync"}` bij start, `{"type":"refresh"}` bij tik.
- Lettertype bril: vermijd °, ·, …, slimme quotes en emoji (telefoon zet ze om; graden worden getekend).

## Testen zonder bril
```sh
node test/phone.cjs Glance.html test/out      # telefoonpagina met nep-data → snapshot.json + screenshot
node test/sim.mjs Glance.html test/out/snapshot.json test/out   # bril-simulator → SVG per pagina
node test/shot.cjs test/out                    # SVG → PNG + overzichtsblad
```
Nog niet op echte bril getest; echte HLN/Yahoo-feeds niet live geverifieerd.
