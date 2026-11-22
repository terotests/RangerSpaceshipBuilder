# RangerSpaceshipBuilder

Avaruusaluksia [RangerGamesin](https://github.com/terotests/RangerGames) peleihin:
web-editori, jolla alus kootaan osista, ja työkalu, joka tekee aluksista
`.glb`-malleja ja RangerGamesin luettavan **alus­paketin**.

Editori selaimessa: <https://terotests.github.io/RangerSpaceshipBuilder/>

## Formaatti

Alus tallennetaan **reseptinä** (`ships/*.ship.json`): lista osia parametreineen
(runko poikkileikkauksina, siivet, moottorit, ohjaamo, tykkitornit, renkaat,
aurinkopaneelit, piikit/lonkerot, orgaaniset möykyt, antennit, ristikkopalkit).
Resepti on pieni (1–3 kt), sitä on helppo muokata käsin tai koodilla, se
näkyy gitissä järkevänä diffinä, ja saman aluksen voi skaalata tai värittää
uudelleen muuttamalla muutamaa lukua. Peilaus ja toisto (`repeat`) tekevät
symmetriset ja säteittäiset rakenteet (asemat) yhdellä osalla.

Peliin alus viedään **glTF-binäärinä (`.glb`)**, jonka RangerGamesin moottori
jo lukee (`GLTFLoader`, `<primitive>`): litteäksi varjostettuja kolmioita,
värit kärkipisteissä (`COLOR_0`), moottorien liekit ja ikkunat omana
valaisemattomana materiaalinaan (`KHR_materials_unlit`). Sama tiedosto aukeaa
Blenderissä ja three.js:ssä. Tarkemmin: [docs/format.md](docs/format.md).

Keula osoittaa **+z**, ylös on **+y**; reseptin `length` kertoo aluksen
oikean pituuden metreinä.

## Valmiit alukset

| | luokka | pituus | kolmioita |
| --- | --- | --- | --- |
| Kipinä | hävittäjä | 14 m | 772 |
| Sirppi | hävittäjä | 17 m | 1104 |
| Ampiainen | elävä alus (hyönteinen) | 12 m | 1872 |
| Muuli | rahtikorvetti | 70 m | 2424 |
| Vartija | fregatti | 150 m | 3032 |
| Valas | elävä emoalus | 420 m | 3136 |
| Vasara | taistelu-emoalus | 680 m | 8224 |
| Kehä | avaruusasema | 1400 m | 4896 |

Editorin **Mittakaava**-valinta näyttää kaikki alukset vierekkäin oikeissa
kokosuhteissa.

## Rakenne

| Polku | Mitä |
| --- | --- |
| `crates/ship_core` | Resepti → mesh → `.glb`, **Rangerin Rust-osajoukolla** (`use ranger::prelude::*`): cargo kääntää sen natiivina ja WebAssemblynä, ja `rgrc` kirjoittaa saman lähteen JavaScriptiksi, C++:ksi, Go:ksi, Pythoniksi … (esim. puhelimeen tai peliin, joka rakentaa aluksia itse) |
| `crates/ship_web` | ship_core WebAssemblynä C ABI:n takana editorille |
| `crates/ship_cli` | `ship glb`, `ship info`, `ship pack` |
| `web/` | Editori: osalista, ominaisuudet, JSON-näkymä, WebGL-näkymä (valinta napauttamalla), kumoa/tee uudelleen, omat alukset selaimen muistiin, lataukset (resepti, .glb, paketti .zip) |
| `ships/` | Valmiit alukset |

## Aloitus

```sh
bash scripts/setup.sh                       # Ranger hakemistoon .deps/ (prelude-crate ja rgrc)
rustup target add wasm32-unknown-unknown
bash scripts/build-web.sh                   # → dist/web
python3 -m http.server -d dist/web 8000     # http://localhost:8000  (?ship=vasara)
```

Testit:

```sh
cargo test --workspace                      # alukset rakentuvat, kolmiobudjetti, glb kunnossa
bash scripts/check-ranger.sh                # ship_core Rangerin osajoukossa + JS tekee saman glb:n
TARGETS="es6 cpp go python" bash scripts/check-ranger.sh
```

## RangerGamesiin

```sh
cargo run --release -p ship_cli -- pack ships ../RangerGames/games/<peli>/ships
```

tekee kansioon `ships.json`-luettelon (ja saman moduulina `ships.ts`),
`<id>.glb`-mallit ja reseptit. Saman paketin saa editorista napilla
**⬇ Paketti**. Pelissä:

```tsx
import { SHIPS } from "ships/ships";
import { GLTFLoader } from "three/addons/loaders/GLTFLoader.js";

const ship = SHIPS[0];
const gltf = new GLTFLoader().loadSync("ships/" + ship.model);
// <scene3d> … <primitive object={gltf.scene} scale={1 / ship.radius} /> …
```

GitHub Actions ajaa testit ja julkaisee editorin (ja valmiin paketin
osoitteeseen `/pack/`) GitHub Pagesiin jokaisesta `main`-pushista.

Lisenssi: MIT.
