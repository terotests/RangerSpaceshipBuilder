# Aluksen resepti (`.ship.json`) ja alus­paketti

## Resepti

```json
{
  "format": "ranger-ship", "version": 1,
  "name": "Kipinä", "class": "fighter", "length": 14, "seed": 7,
  "palette": { "hull": "#d7dbe0", "accent": "#e63946", "dark": "#2f343b", "glow": "#5ad1ff", "glass": "#7fd8ff" },
  "parts": [ { "type": "hull", … }, { "type": "wing", "mirror": true, … } ]
}
```

- `class`: `fighter`, `corvette`, `frigate`, `cruiser`, `capital`, `station`, `creature`
- `length`: oikea pituus metreinä (mallin yksiköt ovat vapaat; peli skaalaa)
- `seed`: kohinan, pintayksityiskohtien ja paneelisävyjen siemen
- `palette`: nimetyt värit (sRGB-heksat); osat viittaavat niihin nimellä tai
  antavat oman `"#rrggbb"`-värin. Oletuksina `hull`, `accent`, `dark`, `glow`,
  `glass`, `solar`; omia nimiä saa lisätä.

### Kaikkien osien yhteiset kentät

| kenttä | |
| --- | --- |
| `type` | osan tyyppi (alla) |
| `name` | nimi editorissa |
| `at`, `rot`, `scale` | paikka, kierto asteina (x, sitten y, sitten z), skaala (luku tai `[x, y, z]`) |
| `mirror` | `true` = lisäksi peilikuva x = 0 -tason yli |
| `repeat` | `{ count, step: [x,y,z], rotate: [x,y,z], scale }`: kopio k kierretään k × rotate aluksen origon ympäri, siirretään k × step ja skaalataan scale^k |
| `color`, `accent` | värit (paletin nimi tai heksa) |
| `glow` | `true` = valaisematon (valot, ikkunat) |
| `hidden` | `true` = jätetään pois |
| `panel` | paneelien sävyvaihtelu 0…0,6 |

### Osat

Osa rakennetaan omassa koordinaatistossaan: +z eteen, +y ylös.

| tyyppi | parametrit |
| --- | --- |
| `hull` | `sections: [[z, leveys, korkeus, pyöreys, dy], …]` hännästä keulaan (pyöreys 2 = soikio, 4+ = laatikko, 1 = vinoneliö); `smooth` (Catmull-Rom-välit), `sides`, `belly` (pohjan litteys), `noise`, `noiseScale`, `ribs`, `ribFreq` (orgaaninen), `under` (pohjan väri), `bands` (raidat leikkausten väleihin), `greebles`, `greebleSize`, `greebleArc`, `greebleColor`, `windows`, `windowSize`, `windowHeight`, `windowColor` |
| `wing` | `span`, `root`, `tip`, `sweep` (kärjen siirto taaksepäin; negatiivinen = eteenpäin), `thick`, `tipThick`, `dihedral`, `tipBand` |
| `engine` | `r`, `len`, `sides`, `nozzle`, `nozzleLen`, `intake`, `glowColor` |
| `cockpit` | `size: [w, h, l]`, `detail`, `flat` |
| `box` | `size`, `taper: [x, y]` (etupään kavennus), `lift`, `bevel` |
| `cylinder` | `r`, `r2` (etusäde), `len`, `sides`, `capColor` |
| `turret` | `r`, `barrels`, `barrelLen`, `barrelR` |
| `ring` | `r`, `thick`, `width`, `segments`, `spokes`, `spokeWidth`, `hub`, `windows` (z-akselin ympäri) |
| `panel` | `size: [w, l]` (xz-tasossa), `cells: [nx, nz]`, `thick`, `gap`, `spine` |
| `spike` | `path: [[x,y,z], …]`, `r0`, `r1`, `sides`, `steps`, `wobble`, `tip` |
| `blob` | `size`, `rings`, `sides`, `noise`, `noiseScale`, `ribs`, `ribFreq` |
| `dish` | `r`, `depth`, `sides` (aukko +z) |
| `truss` | `len`, `width`, `segments`, `thick` (z-suunnassa) |

Puuttuva parametri saa oletusarvon (`crates/ship_core/src/parts.rs`), joten
`{ "type": "engine" }` on jo kelvollinen osa.

## .glb

Yksi solmu (`extras: { class, length }`), yksi mesh, kaksi primitiiviä:
materiaali 0 `hull` (valaistu, värit `COLOR_0`:ssa lineaarisina) ja
materiaali 1 `glow` (`KHR_materials_unlit`, `emissiveFactor` 1). Kolmiot
ovat indeksoimattomia ja litteästi varjostettuja (jokaisella kulmalla
kolmion normaali).

## Alus­paketti

```text
ships/
  ships.json
  kipina.glb   kipina.ship.json
  …
```

```json
{
  "format": "ranger-ship-pack", "version": 1, "forward": "+z", "up": "+y",
  "ships": [
    { "id": "kipina", "name": "Kipinä", "class": "fighter", "length": 14,
      "model": "kipina.glb", "recipe": "kipina.ship.json", "triangles": 772,
      "size": [4.49, 1.25, 5.65], "center": [0, 0.37, 0.07], "radius": 3.7 }
  ]
}
```

`size`, `center` ja `radius` ovat mallin yksiköissä: `length / size[2]`
muuttaa metreiksi, `1 / radius` sovittaa aluksen yksikköpalloon.
