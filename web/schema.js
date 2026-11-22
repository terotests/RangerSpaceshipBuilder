// SPDX-License-Identifier: MIT
// The editor's view of the part types (crates/ship_core/src/parts.rs): each
// parameter's label, kind and range. The defaults here are the ones the Rust
// side uses when a parameter is missing.

export const CLASSES = [
  ["fighter", "Hävittäjä"],
  ["corvette", "Korvetti"],
  ["frigate", "Fregatti"],
  ["cruiser", "Risteilijä"],
  ["capital", "Emoalus"],
  ["station", "Avaruusasema"],
  ["creature", "Elävä alus"],
];

const n = (key, label, def, min, max, step = 0.01) => ({ key, label, kind: "num", def, min, max, step });
const i = (key, label, def, min, max) => ({ key, label, kind: "num", def, min, max, step: 1 });
const b = (key, label, def) => ({ key, label, kind: "bool", def });
const c = (key, label, def) => ({ key, label, kind: "color", def });
const v = (key, label, def, step = 0.05) => ({ key, label, kind: "v3", def, step });

export const PATTERNS = [
  ["plates", "Paneelit"],
  ["armor", "Panssari"],
  ["hex", "Heksat"],
  ["organic", "Orgaaninen"],
  ["carbon", "Hiilikuitu"],
  ["solar", "Kennot"],
  ["vents", "Ritilä"],
  ["glass", "Lasi"],
  ["paint", "Maali"],
  ["hazard", "Varoitusraidat"],
];

// a recipe's own material (crates/ship_core/src/material.rs); the defaults
// depend on the pattern, so these are only where the sliders start
export const MATERIAL_PARAMS = [
  { key: "scale", label: "Kuvion koko", kind: "num", def: 1, min: 0.1, max: 6, step: 0.05 },
  { key: "metal", label: "Metallisuus", kind: "num", def: 0.5, min: 0, max: 1, step: 0.01 },
  { key: "rough", label: "Karheus", kind: "num", def: 0.45, min: 0.02, max: 1, step: 0.01 },
  { key: "grime", label: "Lika ja kuluma", kind: "num", def: 0.25, min: 0, max: 1, step: 0.01 },
  { key: "depth", label: "Syvyys", kind: "num", def: 1, min: 0, max: 3, step: 0.05 },
  { key: "seed", label: "Siemen", kind: "num", def: 1, min: 1, max: 999, step: 1 },
];

export const COMMON = [
  { key: "name", label: "Nimi", kind: "text", def: "" },
  { key: "material", label: "Materiaali", kind: "material", def: "" },
  v("at", "Paikka", [0, 0, 0]),
  v("rot", "Kierto °", [0, 0, 0], 1),
  v("scale", "Skaala", [1, 1, 1], 0.02),
  b("mirror", "Peilaa (x)", false),
  b("glow", "Hehkuu", false),
];

export const REPEAT = [
  i("count", "Kopioita", 1, 1, 64),
  v("step", "Siirto / kopio", [0, 0, 0]),
  v("rotate", "Kierto / kopio °", [0, 0, 0], 1),
  n("scale", "Skaala / kopio", 1, 0.5, 1.5),
];

export const TYPES = {
  hull: {
    label: "Runko",
    params: [
      { key: "sections", label: "Poikkileikkaukset [z, leveys, korkeus, pyöreys, dy]", kind: "sections",
        def: [[-2, 0.8, 0.6, 2.5, 0], [0, 1.2, 0.8, 2.5, 0], [2, 0.05, 0.05, 2, 0]] },
      i("smooth", "Pehmennys", 0, 0, 6),
      i("sides", "Sivuja", 12, 3, 48),
      n("belly", "Pohjan litteys", 1, 0, 1.5),
      n("noise", "Kohina (orgaaninen)", 0, 0, 0.6),
      n("noiseScale", "Kohinan tiheys", 1.5, 0.1, 8, 0.05),
      n("ribs", "Rivat", 0, 0, 0.5),
      n("ribFreq", "Ripojen tiheys", 4, 0.2, 20, 0.1),
      n("panel", "Paneelivaihtelu", 0.12, 0, 0.6),
      c("color", "Väri", "hull"),
      c("accent", "Raitaväri", "accent"),
      c("under", "Pohjan väri", ""),
      { key: "bands", label: "Raidat (leikkausten numerot)", kind: "intlist", def: [] },
      i("greebles", "Pintayksityiskohdat", 0, 0, 400),
      n("greebleSize", "Yksityiskohtien koko", 0.12, 0.01, 2),
      n("greebleArc", "Yksityiskohtien alue", 0.5, 0.05, 1),
      c("greebleColor", "Yksityiskohtien väri", "dark"),
      i("windows", "Ikkunat", 0, 0, 400),
      n("windowSize", "Ikkunan koko", 0.05, 0.005, 1, 0.005),
      n("windowHeight", "Ikkunoiden korkeus", 0, -1, 1),
      c("windowColor", "Ikkunoiden väri", "glow"),
    ],
  },
  wing: {
    label: "Siipi",
    params: [
      n("span", "Kärkiväli", 2, 0.05, 40, 0.05),
      n("root", "Tyven leveys", 1.5, 0.05, 40, 0.05),
      n("tip", "Kärjen leveys", 0.5, 0, 40, 0.05),
      n("sweep", "Nuolikulma", 0.6, -10, 20, 0.05),
      n("thick", "Paksuus", 0.12, 0.01, 4),
      n("tipThick", "Kärjen paksuus ×", 0.5, 0.05, 2),
      n("dihedral", "V-kulma °", 0, -60, 60, 1),
      n("tipBand", "Kärjen raita", 0, 0, 0.9),
      n("panel", "Paneelivaihtelu", 0.08, 0, 0.6),
      c("color", "Väri", "hull"),
      c("accent", "Raitaväri", "accent"),
    ],
  },
  engine: {
    label: "Moottori",
    params: [
      n("r", "Säde", 0.3, 0.01, 20),
      n("len", "Pituus", 1.2, 0.05, 60, 0.05),
      i("sides", "Sivuja", 12, 4, 32),
      n("nozzle", "Suutin ×", 1.15, 0.5, 2),
      n("nozzleLen", "Suuttimen pituus", 0.25, 0.05, 0.8),
      n("intake", "Imuaukko ×", 0.7, 0, 1.2),
      c("color", "Väri", "dark"),
      c("accent", "Renkaat", "hull"),
      c("glowColor", "Liekki", "glow"),
    ],
  },
  cockpit: {
    label: "Ohjaamo",
    params: [
      v("size", "Koko", [0.5, 0.35, 1.0], 0.02),
      i("detail", "Tarkkuus", 10, 4, 24),
      n("flat", "Pohjan litteys", 0.25, 0, 1),
      c("color", "Väri", "glass"),
    ],
  },
  box: {
    label: "Laatikko",
    params: [
      v("size", "Koko", [1, 1, 1], 0.05),
      { key: "taper", label: "Kärjen kavennus [x, y]", kind: "v2", def: [1, 1], step: 0.02 },
      n("lift", "Kärjen nosto", 0, -20, 20),
      n("bevel", "Viiste", 0, 0, 0.49),
      n("panel", "Paneelivaihtelu", 0.1, 0, 0.6),
      c("color", "Väri", "hull"),
    ],
  },
  cylinder: {
    label: "Sylinteri",
    params: [
      n("r", "Säde", 0.5, 0.001, 40),
      n("r2", "Etusäde", 0.5, 0, 40),
      n("len", "Pituus", 1, 0.01, 200, 0.05),
      i("sides", "Sivuja", 12, 3, 48),
      c("color", "Väri", "hull"),
      c("capColor", "Päiden väri", ""),
    ],
  },
  turret: {
    label: "Tykkitorni",
    params: [
      n("r", "Koko", 0.3, 0.02, 10),
      i("barrels", "Piippuja", 2, 1, 6),
      n("barrelLen", "Piipun pituus", 0.8, 0.05, 20, 0.05),
      n("barrelR", "Piipun säde", 0.035, 0.005, 2, 0.005),
      c("color", "Väri", "hull"),
      c("accent", "Piiput", "dark"),
    ],
  },
  ring: {
    label: "Rengas",
    params: [
      n("r", "Säde", 4, 0.2, 400, 0.1),
      n("thick", "Paksuus", 0.5, 0.01, 50),
      n("width", "Leveys", 0.8, 0.01, 50),
      i("segments", "Lohkoja", 24, 6, 96),
      i("spokes", "Puolia", 4, 0, 16),
      n("spokeWidth", "Puolan paksuus", 0.18, 0.01, 10),
      n("hub", "Navan säde", 0.6, 0, 100),
      b("windows", "Ikkunarivit", true),
      c("color", "Väri", "hull"),
      c("accent", "Ikkunat / raita", "accent"),
    ],
  },
  panel: {
    label: "Aurinkopaneeli",
    params: [
      { key: "size", label: "Koko [leveys, pituus]", kind: "v2", def: [2, 1], step: 0.05 },
      { key: "cells", label: "Kennot [x, z]", kind: "v2", def: [4, 2], step: 1 },
      n("thick", "Paksuus", 0.03, 0.005, 2, 0.005),
      n("gap", "Rako", 0.04, 0, 2, 0.005),
      b("spine", "Tukipalkki", true),
      c("color", "Kennot", "solar"),
      c("accent", "Kehys", "hull"),
    ],
  },
  spike: {
    label: "Piikki / lonkero",
    params: [
      { key: "path", label: "Reitti [x, y, z]", kind: "path", def: [[0, 0, 0], [0, 0.3, 0.6], [0, 0.4, 1.4]] },
      n("r0", "Tyven säde", 0.15, 0.001, 20),
      n("r1", "Kärjen säde", 0, 0, 20),
      i("sides", "Sivuja", 6, 3, 24),
      i("steps", "Pehmennys", 4, 1, 16),
      n("wobble", "Aaltoilu", 0, 0, 0.8),
      n("tip", "Kärjen väriosuus", 0, 0, 1),
      c("color", "Väri", "hull"),
      c("accent", "Kärki", "accent"),
    ],
  },
  blob: {
    label: "Möykky (orgaaninen)",
    params: [
      v("size", "Koko", [1, 0.7, 1.6], 0.05),
      i("rings", "Renkaita", 8, 3, 32),
      i("sides", "Sivuja", 12, 4, 48),
      n("noise", "Kohina", 0.15, 0, 0.8),
      n("noiseScale", "Kohinan tiheys", 1.2, 0.1, 8, 0.05),
      n("ribs", "Rivat", 0, 0, 0.5),
      n("ribFreq", "Ripojen tiheys", 4, 0.2, 20, 0.1),
      c("color", "Väri", "hull"),
    ],
  },
  dish: {
    label: "Antennilautanen",
    params: [
      n("r", "Säde", 0.8, 0.05, 40),
      n("depth", "Syvyys", 0.25, 0.02, 1),
      i("sides", "Sivuja", 16, 6, 48),
      c("color", "Väri", "hull"),
      c("accent", "Tausta", "dark"),
    ],
  },
  truss: {
    label: "Ristikkopalkki",
    params: [
      n("len", "Pituus", 4, 0.1, 400, 0.1),
      n("width", "Leveys", 0.4, 0.02, 40),
      i("segments", "Välejä", 6, 1, 64),
      n("thick", "Palkin paksuus", 0.04, 0.002, 4, 0.002),
      c("color", "Väri", "hull"),
    ],
  },
};

/// A new part of `type` with sensible starting values.
export function newPart(type) {
  const p = { type, name: TYPES[type].label };
  for (const d of TYPES[type].params) {
    if (d.kind === "sections" || d.kind === "path" || d.kind === "v2" || d.kind === "v3") p[d.key] = JSON.parse(JSON.stringify(d.def));
  }
  if (type === "wing" || type === "engine") p.mirror = true;
  if (type === "wing") p.at = [0.4, 0, 0];
  if (type === "engine") p.at = [0.4, 0, -2];
  return p;
}
