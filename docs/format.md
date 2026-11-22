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

- `material`: osien oletusmateriaali (muuten osatyypin mukaan: runko, siivet,
  laatikot ja renkaat `plates`, ohjaamo `glass`, moottori `vents`, tykkitorni
  `armor`, paneeli `solar`, muut `paint`)
- `materials`: omat materiaalit, ks. alla
- `textureSize`: tekstuurien koko pikseleinä (64…1024, oletus 256)

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
| `material` | materiaali: valmis kuvio tai reseptin oma materiaali |
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

## Materiaalit ja tekstuurit

Materiaali on kuvio ja pinnan valo-ominaisuudet. Valmiit materiaalit on
nimetty kuvioidensa mukaan:

| kuvio | |
| --- | --- |
| `plates` | paneelit eri kokoisina riveinä, saumat, niitit, luukut |
| `armor` | isot viistetyt panssarilevyt, naarmut |
| `hex` | kuusikulmiolaatat (myös kitiini, energiakilpi) |
| `organic` | soluja ja suonia (Voronoi), märkä pinta |
| `carbon` | hiilikuitukudos |
| `solar` | aurinkokennot ja johdinviivat |
| `vents` | ritilät ja säleiköt |
| `glass` | lähes sileä lasi |
| `paint` | maalipinta, jossa lohkeamia (alta paljastuu metallia) |
| `hazard` | vinot varoitusraidat |

Oma materiaali:

```json
"materials": {
  "raskas": { "pattern": "armor", "scale": 1.4, "metal": 0.8, "rough": 0.35, "grime": 0.5, "depth": 1, "seed": 1 }
}
```

`scale` on kuvion koko (yksi ruutu on pienellä aluksella noin 2,5 m ja isolla
1/14 aluksen pituudesta, kerrottuna tällä), `metal` ja `rough` metallisuus ja
karheus, `grime` lika, valumajäljet ja kuluneet reunat, `depth` saumojen
syvyys normaalikartassa.

Tekstuurit tehdään ship_coressa (`texture.rs`) ja ne toistuvat saumatta:
värikartta on harmaasävyinen ja kerrotaan osan värillä (paletti), joten sama
materiaali käy kaikille väreille. Lisäksi metalli/karheuskartta (glTF:n
järjestys: G karheus, B metalli) ja normaalikartta korkeuskentästä. UV:t
projisoidaan kunkin kolmion pääakselin suuntaan osan omassa koordinaatistossa.

## .glb

Yksi solmu (`extras: { class, length }`), yksi mesh, primitiivi jokaiselle
materiaalille (valaistu, värit `COLOR_0`:ssa lineaarisina, `TEXCOORD_0`) ja
viimeisenä `glow` (`KHR_materials_unlit`, `emissiveFactor` 1). Kolmiot
ovat indeksoimattomia ja litteästi varjostettuja (jokaisella kulmalla
kolmion normaali).

Tekstuureineen (`ship glb --textures`, editorin ⬇ .glb) jokaisella
materiaalilla on `baseColorTexture`, `metallicRoughnessTexture` ja
`normalTexture` (PNG:t samassa tiedostossa). Ilman tekstuureja (`ship pack`
oletuksena) materiaaleilla on vain `metallicFactor` ja `roughnessFactor`:
RangerGamesin moottori ei vielä lue tekstuureja, joten pelissä alus näyttää
samalta kuin editorissa ilman **Tekstuurit**-valintaa.

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

## Moottorin oma 3D-kerros (`ship scene`)

`ship scene <resepti> <out.ts> [pituus]` kirjoittaa aluksen TypeScript-moduuliksi
RangerGamesin `rg.three("add", { shape: "mesh", ... })`-kutsulle (scene3d.rs,
esim. Solar Pilot): `verts` (hitsatut kulmat, 1/1000 yksikön tarkkuus),
`tris` (indeksit, vastapäivään ulkoa katsottuna), `colors` (0xrrggbb, sRGB,
yksi kolmiota kohti) ja `glow` (valaisevien kolmioiden numerot). Malli on
keskitetty, käännetty niin että keula osoittaa −z:aan, ja skaalattu `pituus`
yksikön pituiseksi (oletus 2). Tekstuureja tässä muodossa ei ole.
