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
- `Glance/companion.html` – telefoon: haalt Open-Meteo, iCal-links, RSS (standaard HLN, met reserve-URL), Yahoo Finance (standaard IWDA:AMS = IWDA.AS en BTC-EUR; notatie TICKER:BEURS wordt omgezet) op.
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

# Wegwijs – navigatie-app voor Mira-glasses (tweede app in deze repo)

Aparte app naast Glance; verschijnt als eigen kaart in de Mira-app en als eigen item in de zijbalk.

## Bouwen en valideren
```sh
python3 Wegwijs/package.py Wegwijs.html
python3 Wegwijs/validate.py Wegwijs.html
```
Lever alleen `Wegwijs.html` op (zelfde installatiestappen als Glance).

## Structuur
- `Wegwijs/wegwijs.rs` – bril-app (app-naam "Wegwijs"). Schermen: navigatie (vectorkaart koers-boven met route, straten,
  positie-chevron, pijlpaneel rechtsboven met afstand/straat/"daarna", voetbalk met resterende afstand, tijd, aankomst,
  klok en batterij), stappenlijst (swipe), overzicht van de hele route (tik; automatisch 8 s bij een nieuwe route),
  restaurant-keuzekaarten (swipe = volgende, tik = navigeer), aangekomen-scherm, idle-scherm.
- `Wegwijs/companion.html` – telefoon: bestemming zoeken (Google Places met sleutel, anders Photon, dan Nominatim),
  favorieten/recent, kiezen op kaart, routering (Valhalla standaard, Google Routes met sleutel, OSRM reserve) voor
  te voet / fiets / auto, gps-volgen met koers (gps-koers, traject, kompas), voortgang, afslag-aankondigingen via
  `speechSynthesis` (instelbaar), herberekenen bij afwijken, aankomst, canvas-kaart met OSM-tegels, Overpass-straten
  voor de bril, restaurantzoeker (Tripadvisor Content API met sleutel, Google Places met sleutel, anders
  OpenStreetMap zonder beoordelingen) met filters op sterren, afstand, keuken en "nu open", demo-wandeling.
- Protocol telefoon → bril (JSON `{type, d}`, `d` = regels `TAG|veld|…`, frame = meters t.o.v. de eerste gps-fix,
  x oost / y noord, punten delta-gecodeerd als zigzag-varints met 5 bits per teken in `A-Za-z0-9-_`):
  - `nav` (snapshot, ~1/s): `T|status|tekst` (0 idle, 1 route berekenen, 2 onderweg, 3 aangekomen, 4 van de route,
    5 geen gps, 6 fout), `S|x|y|koers|nauwkeurigheid|beweegt`, `M|code|afstand|straat|rotonde-afslag`,
    `A|code|straat` (daarna), `R|rest_m|rest_min|aankomst|totaal_m`, `D|bestemming`, `P|stap|segment`, `V|modus|draait`.
  - `route` (geordend): `F|id|overzicht`, `G|punten`, `K|puntindex|code|lengte|afslag|tekst` per stap, `D|naam|m|min`.
  - `streets` (geordend): `L|punten` per straat. `places` (geordend): `F|n`, `Q|naam|sterren×10|aantal|info|afstand|open|adres`.
  - `clear`. Manoeuvre-codes: S rechtdoor, L/R links/rechts, l/r flauw, H/h scherp, U/u keren, K/k aanhouden,
    O rotonde, A bestemming, D vertrek, T trap, F veerboot.
  - `toast`: één regel tekst, 5 s onderaan op de bril. `places` heeft `F|n|soort` (r restaurants, a bezienswaardigheden) en
    `Q|…|adres|ranking|uren vandaag|omschrijving`; op de bril eerst een lijst (swipe = kiezen, tik = detailkaart,
    tik = navigeer, lang indrukken = terug/sluiten).
- Bril → telefoon: `{"type":"sync"}`, `{"type":"go","i":n}` (plaats gekozen), `{"type":"close"}`, `{"type":"done"}`,
  `{"type":"say","text":"…"}` (ingesproken zin).
- Spraak op de bril: de app claimt holds (`content::holds_local`). Touchbar ingedrukt houden start de SDK-transcriptie
  (`ble_request` 0x0001, snapshot 0x0003 elke 250 ms, stop 0x0002), loslaten stuurt de zin naar de telefoon.
  `handleCommand` in de companion herkent reismodus ("wandel/fiets/rij"), "naar <plaats>", "zoek … restaurant/
  bezienswaardigheden" met keuken, sterren, afstand en "open", en "stop"/"herbereken". Antwoorden komen als `toast` en spraak.

## Testen zonder bril
```sh
node test/wegwijs-phone.cjs Wegwijs.html test/out   # telefoon met nep-gps en opgenomen API-antwoorden → messages.json, marks.json, screenshots
node test/wegwijs-sim.mjs Wegwijs.html test/out test/out/sim   # bril-simulator → SVG per scherm
node test/shot.cjs test/out/sim                       # SVG → PNG + overzichtsblad
```
Fixtures in `test/fixtures/` zijn echte antwoorden van Valhalla, OSRM, Photon en Nominatim (Gent); Overpass en
Tripadvisor zijn in de test synthetisch. Nog niet op een echte bril getest; Tripadvisor/Google alleen met eigen sleutel.
