// SPDX-License-Identifier: MIT
// The ship editor: a recipe (.ship.json) on the left and right, the ship
// built by ship_core (WebAssembly) in the middle.

import { View3D, LIGHTS } from "./view3d.js";
import { TYPES, COMMON, REPEAT, CLASSES, PATTERNS, MATERIAL_PARAMS, newPart } from "./schema.js";
import { zip } from "./zip.js";

const $ = (id) => document.getElementById(id);
const el = (tag, props = {}, ...kids) => {
  const e = document.createElement(tag);
  for (const k in props) {
    if (k === "class") e.className = props[k];
    else if (k.startsWith("on")) e.addEventListener(k.slice(2), props[k]);
    else if (k in e && typeof props[k] !== "string") e[k] = props[k];
    else e.setAttribute(k, props[k]);
  }
  for (const c of kids) if (c != null) e.append(c);
  return e;
};
const clone = (o) => JSON.parse(JSON.stringify(o));
const PALETTE_KEYS = ["hull", "accent", "dark", "glow", "glass", "solar"];
const PALETTE_NAMES = { hull: "Runko", accent: "Raita", dark: "Tumma", glow: "Hehku", glass: "Lasi", solar: "Paneeli" };
const DEFAULT_PALETTE = { hull: "#b8bec7", accent: "#d94848", dark: "#3a3f47", glow: "#62d6ff", glass: "#86e0ff", solar: "#2b4f9e" };

// ---- ship_core as WebAssembly --------------------------------------------------

let wasm = null;
async function loadWasm() {
  const res = await fetch("ship_web.wasm");
  const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
  wasm = instance.exports;
}

function build(recipe) {
  const text = new TextEncoder().encode(typeof recipe === "string" ? recipe : JSON.stringify(recipe));
  const p = wasm.ship_alloc(text.length);
  new Uint8Array(wasm.memory.buffer, p, text.length).set(text);
  wasm.ship_build(p, text.length);
  wasm.ship_free(p, text.length);
  const info = JSON.parse(new TextDecoder().decode(new Uint8Array(wasm.memory.buffer, wasm.ship_info_ptr(), wasm.ship_info_len())));
  const mesh = new Float32Array(wasm.memory.buffer, wasm.ship_mesh_ptr(), wasm.ship_mesh_len()).slice();
  const r = new Uint32Array(wasm.memory.buffer, wasm.ship_ranges_ptr(), wasm.ship_ranges_len());
  const ranges = [];
  for (let i = 0; i < r.length; i += 2) ranges.push([r[i], r[i + 1]]);
  return { info, mesh, ranges };
}

/// The materials of the recipe just built, with their GL textures (made by
/// ship_core once per material definition and kept).
function materialsOf(recipe, info) {
  const obj = typeof recipe === "string" ? JSON.parse(recipe) : recipe;
  return info.materials.map((m, i) => {
    const key = JSON.stringify([m.name, m.pattern, (obj.materials || {})[m.name] || null, info.textureSize]);
    const tex = view.materialTex(key, () => {
      const size = wasm.ship_tex(i);
      if (!size) return null;
      return { size, data: new Uint8Array(wasm.memory.buffer, wasm.ship_tex_ptr(), size * size * 9).slice() };
    });
    return { ...m, tex };
  });
}

/// A built ship as the viewer wants it.
function meshOf(recipe, extra) {
  const b = build(recipe);
  return { b, item: { data: b.mesh, ranges: b.ranges, mats: materialsOf(recipe, b.info), ...extra } };
}

function glbOf(recipe, textures) {
  build(recipe);
  const n = wasm.ship_glb(textures ? 1 : 0);
  return new Uint8Array(wasm.memory.buffer, wasm.ship_glb_ptr(), n).slice();
}

// ---- state ---------------------------------------------------------------------

const presets = (window.SHIP_PRESETS || []).map((p) => ({ id: p.id, recipe: p.recipe, preset: true }));
let mine = {};
try { mine = JSON.parse(localStorage.getItem("ships.mine") || "{}"); } catch (e) { mine = {}; }

let current = { id: "", preset: false };
let recipe = null;
let selected = -1;
let history = [], future = [];
let lastBuilt = null;
let view;

function allShips() {
  const list = presets.map((p) => ({ id: p.id, recipe: p.recipe, preset: true }));
  for (const id in mine) list.push({ id, recipe: mine[id], preset: false });
  return list;
}

function blankShip() {
  return {
    format: "ranger-ship", version: 1, name: "Uusi alus", class: "fighter", length: 15, seed: 1,
    palette: { ...DEFAULT_PALETTE },
    parts: [newPart("hull")],
  };
}

function slug(s) {
  const map = { ä: "a", ö: "o", å: "a", Ä: "a", Ö: "o", Å: "a" };
  return (s || "alus").replace(/[äöåÄÖÅ]/g, (c) => map[c]).toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") || "alus";
}

function commit(next, keepFuture) {
  history.push(JSON.stringify(recipe));
  if (history.length > 200) history.shift();
  if (!keepFuture) future = [];
  recipe = next;
  refresh();
}

/// A change to the recipe in place (sliders): one undo step per gesture.
let pendingUndo = null;
function change(fn) {
  if (pendingUndo === null) {
    pendingUndo = JSON.stringify(recipe);
    history.push(pendingUndo);
    future = [];
    setTimeout(() => { pendingUndo = null; }, 600);
  }
  fn(recipe);
  scheduleRebuild();
}

let rebuildQueued = false;
function scheduleRebuild() {
  if (rebuildQueued) return;
  rebuildQueued = true;
  requestAnimationFrame(() => { rebuildQueued = false; rebuild(false); renderParts(); syncJson(); });
}

function undo() {
  if (!history.length) return;
  future.push(JSON.stringify(recipe));
  recipe = JSON.parse(history.pop());
  pendingUndo = null;
  refresh();
}
function redo() {
  if (!future.length) return;
  history.push(JSON.stringify(recipe));
  recipe = JSON.parse(future.pop());
  refresh();
}

function load(ship) {
  current = { id: ship.id, preset: ship.preset };
  recipe = clone(ship.recipe);
  history = []; future = [];
  selected = -1;
  refresh(true);
}

// ---- building and drawing ------------------------------------------------------

function rebuild(frame) {
  if ($("chkLineup").checked) { lineup(); return; }
  const t0 = performance.now();
  const { b, item } = meshOf(recipe);
  lastBuilt = b;
  view.setMeshes([item]);
  view.selected = selected;
  if (frame) view.frame(b.info.min, b.info.max);
  const ms = (performance.now() - t0).toFixed(1);
  const tris = b.info.triangles;
  const budget = { fighter: 1500, corvette: 2500, frigate: 3500, creature: 4000 }[recipe.class] || 8000;
  const st = $("status");
  st.innerHTML = "";
  st.append(`${tris} kolmiota · ${b.info.parts} osaa · ${(b.info.max[2] - b.info.min[2]).toFixed(2)} yks. = ${recipe.length} m · ${ms} ms`);
  if (tris > budget) st.append(el("span", { class: "warn" }, ` · paljon kolmioita puhelimelle (suositus < ${budget})`));
  if (b.info.error) st.append(el("span", { class: "err" }, " · " + b.info.error));
  for (const w of b.info.warnings) st.append(el("span", { class: "warn" }, " · " + w));
  $("lineupLabels").innerHTML = "";
}

/// Every ship side by side at its real size (length in metres).
function lineup() {
  const ships = allShips().map((s) => {
    const r = s.id === current.id ? recipe : s.recipe;
    const { b, item } = meshOf(r);
    const len = Math.max(0.001, b.info.max[2] - b.info.min[2]);
    return { name: r.name, length: r.length || 10, b, item, k: (r.length || 10) / len };
  }).sort((a, b) => a.length - b.length);
  let x = 0;
  const items = [];
  let maxH = 0;
  for (const s of ships) {
    const w = (s.b.info.max[0] - s.b.info.min[0]) * s.k;
    const cx = x + w / 2 - ((s.b.info.max[0] + s.b.info.min[0]) / 2) * s.k;
    items.push({ ...s.item, scale: s.k, offset: [cx, -s.b.info.min[1] * s.k, -((s.b.info.max[2] + s.b.info.min[2]) / 2) * s.k] });
    x += w + Math.max(5, s.length * 0.08);
    maxH = Math.max(maxH, (s.b.info.max[1] - s.b.info.min[1]) * s.k, s.length);
  }
  view.selected = -1;
  view.setMeshes(items);
  view.frame([0, 0, -maxH / 2], [x, maxH * 0.6, maxH / 2]);
  view.yaw = 0.25; view.pitch = 0.25; view.dirty = true;
  const lab = $("lineupLabels");
  lab.innerHTML = "";
  for (const s of ships) lab.append(el("span", {}, `${s.name} ${s.length} m`));
  $("status").textContent = `${ships.length} alusta oikeassa mittakaavassa (vasemmalta pienimmästä)`;
}

// ---- the left side: ship and parts ---------------------------------------------

function renderShipProps() {
  const box = $("shipProps");
  box.innerHTML = "";
  box.append(field("Nimi", el("input", { type: "text", value: recipe.name || "", oninput: (e) => change((r) => { r.name = e.target.value; }) }), true));
  const cls = el("select", { onchange: (e) => commit({ ...recipe, class: e.target.value }) });
  for (const [k, label] of CLASSES) cls.append(el("option", { value: k, selected: recipe.class === k }, label));
  box.append(field("Luokka", cls, true));
  box.append(numField({ key: "length", label: "Pituus (m)", def: 15, min: 1, max: 5000, step: 1 }, recipe, (v) => change((r) => { r.length = v; })));
  box.append(numField({ key: "seed", label: "Siemen", def: 1, min: 1, max: 9999, step: 1 }, recipe, (v) => change((r) => { r.seed = v; })));
  const pal = el("div", { class: "palette" });
  recipe.palette = recipe.palette || {};
  for (const k of PALETTE_KEYS) {
    const val = recipe.palette[k] || DEFAULT_PALETTE[k];
    pal.append(el("label", {}, el("input", { type: "color", value: val, oninput: (e) => change((r) => { r.palette[k] = e.target.value; }) }), PALETTE_NAMES[k]));
  }
  box.append(el("div", { class: "field wide" }, el("label", { class: "plain" }, "Paletti"), pal));
}

function renderParts() {
  const ul = $("partList");
  ul.innerHTML = "";
  recipe.parts.forEach((p, i) => {
    const t = TYPES[p.type];
    const li = el("li", { class: (i === selected ? "sel " : "") + (p.hidden ? "hidden" : ""), onclick: () => select(i) },
      el("span", { class: "eye", title: "Näytä / piilota", onclick: (e) => { e.stopPropagation(); commit(toggleHidden(i)); } }, p.hidden ? "◌" : "●"),
      el("span", { class: "label" }, p.name || (t ? t.label : p.type)),
      el("span", { class: "type" }, (t ? t.label : p.type) + (p.mirror ? " ⇋" : "") + (p.repeat && p.repeat.count > 1 ? " ×" + p.repeat.count : "")));
    ul.append(li);
  });
}

function toggleHidden(i) {
  const r = clone(recipe);
  if (r.parts[i].hidden) delete r.parts[i].hidden; else r.parts[i].hidden = true;
  return r;
}

function select(i) {
  selected = i;
  view.selected = i;
  view.dirty = true;
  renderParts();
  renderInspector();
}

// ---- the right side: the selected part -----------------------------------------

function field(label, ctl, plain) {
  return el("div", { class: "field" }, el("label", { class: plain ? "plain" : "" }, label), el("div", { class: "ctl" }, ctl));
}

function fmt(v) { return Math.round(v * 10000) / 10000; }

/// Dragging a label sideways changes the number (like Blender).
function scrub(label, get, set, step) {
  label.addEventListener("pointerdown", (e) => {
    const x0 = e.clientX, v0 = get();
    label.setPointerCapture(e.pointerId);
    const move = (m) => set(fmt(v0 + Math.round((m.clientX - x0) / 3) * step * (m.shiftKey ? 0.1 : 1)));
    const up = () => { label.removeEventListener("pointermove", move); label.removeEventListener("pointerup", up); };
    label.addEventListener("pointermove", move);
    label.addEventListener("pointerup", up);
  });
}

function numField(d, obj, set) {
  const v = obj[d.key] ?? d.def;
  const num = el("input", { type: "number", value: v, step: d.step, class: "short" });
  const range = el("input", { type: "range", min: d.min, max: d.max, step: d.step, value: v });
  num.oninput = () => { const x = parseFloat(num.value); if (!isNaN(x)) { range.value = x; set(x); } };
  range.oninput = () => { num.value = range.value; set(parseFloat(range.value)); };
  const f = field(d.label, el("span", { class: "ctl" }, range, num));
  scrub(f.firstChild, () => parseFloat(num.value) || 0, (x) => { num.value = x; range.value = x; set(x); }, d.step);
  return f;
}

function vecField(d, obj, set, n) {
  const cur = Array.isArray(obj[d.key]) ? obj[d.key].slice() : (typeof obj[d.key] === "number" ? Array(n).fill(obj[d.key]) : d.def.slice());
  const inputs = cur.map((v, k) => el("input", { type: "number", value: v, step: d.step, oninput: (e) => {
    const x = parseFloat(e.target.value); if (isNaN(x)) return; cur[k] = x; set(cur.slice());
  } }));
  const f = field(d.label, el("span", { class: "ctl" }, ...inputs));
  // dragging the label moves all (scale) or x (others)
  scrub(f.firstChild, () => cur[0], (x) => {
    const dx = x - cur[0];
    for (let k = 0; k < n; k++) if (d.key === "scale" || k === 0) { cur[k] = fmt(cur[k] + dx); inputs[k].value = cur[k]; }
    set(cur.slice());
  }, d.step);
  return f;
}

function colorField(d, obj, set) {
  const v = obj[d.key] ?? d.def;
  const sel = el("select");
  sel.append(el("option", { value: "" }, d.def === "" ? "(ei)" : "(oletus: " + (PALETTE_NAMES[d.def] || d.def) + ")"));
  for (const k of PALETTE_KEYS) sel.append(el("option", { value: k, selected: obj[d.key] === k }, PALETTE_NAMES[k]));
  const isHex = typeof obj[d.key] === "string" && obj[d.key].startsWith("#");
  sel.append(el("option", { value: "#", selected: isHex }, "oma väri…"));
  const resolved = (val) => (val && val.startsWith("#")) ? val : (recipe.palette[val] || DEFAULT_PALETTE[val] || "#888888");
  const pick = el("input", { type: "color", class: "swatch", value: resolved(v || d.def) });
  sel.onchange = () => {
    if (sel.value === "#") { set(pick.value); }
    else if (sel.value === "") { set(undefined); pick.value = resolved(d.def); }
    else { set(sel.value); pick.value = resolved(sel.value); }
  };
  pick.oninput = () => { sel.value = "#"; set(pick.value); };
  return field(d.label, el("span", { class: "ctl" }, sel, pick), true);
}

/// A material: the default, a built-in pattern or one of the recipe's own.
function materialField(d, obj, set, defaultLabel) {
  const sel = el("select", { onchange: () => set(sel.value || undefined) });
  sel.append(el("option", { value: "" }, defaultLabel || "(oletus)"));
  const own = Object.keys(recipe.materials || {});
  if (own.length) {
    const g = el("optgroup", { label: "Omat" });
    for (const k of own) g.append(el("option", { value: k, selected: obj[d.key] === k }, k));
    sel.append(g);
  }
  const g2 = el("optgroup", { label: "Valmiit" });
  for (const [k, label] of PATTERNS) g2.append(el("option", { value: k, selected: obj[d.key] === k }, label));
  sel.append(g2);
  return field(d.label, sel, true);
}

function renderMaterials() {
  const box = $("matProps");
  box.innerHTML = "";
  box.append(materialField({ key: "material", label: "Oletus" }, recipe, (v) => { const r = clone(recipe); setParam(r, "material", v); commit(r); }, "(osan mukaan)"));
  const mats = recipe.materials || {};
  for (const name of Object.keys(mats)) {
    const m = mats[name];
    const g = el("div", { class: "group" });
    const pat = el("select", { onchange: () => { const r = clone(recipe); r.materials[name].pattern = pat.value; commit(r); } });
    for (const [k, label] of PATTERNS) pat.append(el("option", { value: k, selected: (m.pattern || "plates") === k }, label));
    g.append(el("h3", {}, name, " ", el("button", { title: "Poista materiaali", onclick: () => {
      const r = clone(recipe); delete r.materials[name]; if (!Object.keys(r.materials).length) delete r.materials; commit(r);
    } }, "×")));
    g.append(field("Kuvio", pat, true));
    for (const d of MATERIAL_PARAMS) g.append(paramField(d, (r) => r.materials[name], (r, k, v) => setParam(r.materials[name], k, v)));
    box.append(g);
  }
  box.append(el("div", { class: "row" }, el("button", { onclick: () => {
    const r = clone(recipe);
    r.materials = r.materials || {};
    let i = 1;
    while (r.materials["materiaali" + i]) i++;
    r.materials["materiaali" + i] = { pattern: "plates", scale: 1, metal: 0.6, rough: 0.4, grime: 0.3 };
    commit(r);
  } }, "+ Oma materiaali")));
}

function tableField(d, obj, set, cols) {
  const rows = clone(obj[d.key] ?? d.def);
  const wrap = el("div", { class: "field wide" }, el("label", { class: "plain" }, d.label));
  const table = el("table", { class: "grid" });
  const head = el("tr");
  for (const c of cols) head.append(el("th", {}, c));
  head.append(el("th"));
  table.append(head);
  rows.forEach((row, ri) => {
    const tr = el("tr");
    cols.forEach((c, ci) => {
      tr.append(el("td", {}, el("input", { type: "number", step: 0.05, value: row[ci] ?? "", oninput: (e) => {
        const x = parseFloat(e.target.value); if (isNaN(x)) return; rows[ri][ci] = x; set(clone(rows));
      } })));
    });
    tr.append(el("td", {}, el("button", { title: "Poista rivi", onclick: () => { rows.splice(ri, 1); commitPart(d.key, rows); } }, "×")));
    table.append(tr);
  });
  wrap.append(table);
  wrap.append(el("button", { onclick: () => {
    const last = rows[rows.length - 1] || cols.map(() => 0);
    const next = last.slice();
    next[cols.length === 3 ? 2 : 0] = (next[cols.length === 3 ? 2 : 0] || 0) + 0.5;
    rows.push(next);
    commitPart(d.key, rows);
  } }, "+ rivi"));
  return wrap;
}

function commitPart(key, value) {
  const r = clone(recipe);
  r.parts[selected][key] = value;
  commit(r);
}

function setParam(obj, key, value) {
  if (value === undefined) delete obj[key]; else obj[key] = value;
}

function paramField(d, getObj, onSet) {
  const obj = getObj(recipe) || {};
  const set = (v) => change((r) => { onSet(r, d.key, v); });
  switch (d.kind) {
    case "num": return numField(d, obj, set);
    case "v3": return vecField(d, obj, set, 3);
    case "v2": return vecField(d, obj, set, 2);
    case "bool": return field(d.label, el("input", { type: "checkbox", checked: !!(obj[d.key] ?? d.def), onchange: (e) => commitPart(d.key, e.target.checked) }), true);
    case "text": return field(d.label, el("input", { type: "text", value: obj[d.key] ?? "", oninput: (e) => set(e.target.value) }), true);
    case "color": return colorField(d, obj, set);
    case "material": return materialField(d, obj, (v) => { const r = clone(recipe); setParam(r.parts[selected], d.key, v); commit(r); });
    case "sections": return tableField(d, obj, set, ["z", "lev.", "kork.", "pyör.", "dy"]);
    case "path": return tableField(d, obj, set, ["x", "y", "z"]);
    case "intlist": return field(d.label, el("input", { type: "text", value: (obj[d.key] || []).join(", "), onchange: (e) => {
      const v = e.target.value.split(/[ ,]+/).filter((s) => s !== "").map(Number).filter((x) => !isNaN(x)); commitPart(d.key, v);
    } }), true);
  }
  return el("div");
}

function renderInspector() {
  const box = $("tab-inspector");
  box.innerHTML = "";
  if (selected < 0 || selected >= recipe.parts.length) {
    box.append(el("p", { class: "hint" }, "Valitse osa vasemmalta tai napauta sitä 3D-näkymässä."));
    return;
  }
  const part = recipe.parts[selected];
  const t = TYPES[part.type];
  box.append(el("h2", {}, t ? t.label : part.type));
  const partOf = (r) => r.parts[selected];
  const onPart = (r, k, v) => setParam(r.parts[selected], k, v);
  for (const d of COMMON) box.append(paramField(d, partOf, onPart));
  if (t) {
    const g = el("div", { class: "group" }, el("h3", {}, "Muoto"));
    for (const d of t.params) g.append(paramField(d, partOf, onPart));
    box.append(g);
  }
  const rg = el("div", { class: "group" }, el("h3", {}, "Toisto"));
  const repOf = (r) => r.parts[selected].repeat || {};
  const onRep = (r, k, v) => {
    const p = r.parts[selected];
    p.repeat = p.repeat || {};
    setParam(p.repeat, k, v);
    if (!p.repeat.count || p.repeat.count <= 1) { if (Object.keys(p.repeat).every((x) => x === "count")) delete p.repeat; }
  };
  for (const d of REPEAT) rg.append(paramField(d, repOf, onRep));
  box.append(rg);
}

// ---- JSON tab ------------------------------------------------------------------

function syncJson() {
  if (document.activeElement === $("jsonText")) return;
  $("jsonText").value = pretty(recipe);
}

/// JSON with short arrays on one line.
function pretty(r) {
  return JSON.stringify(r, null, 2).replace(/\[\s+([-\d.,\s]+?)\s+\]/g, (m, inner) => "[" + inner.split(/,\s*/).join(", ") + "]")
    .replace(/\[\s+(\[[-\d., ]+\](?:,\s+\[[-\d., ]+\])*)\s+\]/g, (m, inner) => "[" + inner.replace(/,\s+\[/g, ", [") + "]");
}

// ---- files ---------------------------------------------------------------------

function download(name, data, type) {
  const blob = data instanceof Blob ? data : new Blob([data], { type });
  const a = el("a", { href: URL.createObjectURL(blob), download: name });
  document.body.append(a); a.click(); a.remove();
  setTimeout(() => URL.revokeObjectURL(a.href), 2000);
}

function packFiles(ships) {
  const files = [];
  const entries = [];
  for (const s of ships) {
    const id = s.id;
    const b = build(s.recipe);
    const glb = glbOf(s.recipe, $("chkPackTex").checked);
    const size = [0, 1, 2].map((k) => b.info.max[k] - b.info.min[k]);
    const center = [0, 1, 2].map((k) => (b.info.max[k] + b.info.min[k]) / 2);
    entries.push({ id, name: s.recipe.name, class: s.recipe.class, length: s.recipe.length, model: id + ".glb", recipe: id + ".ship.json",
      triangles: b.info.triangles, size: size.map(fmt), center: center.map(fmt), radius: fmt(Math.hypot(...size) / 2) });
    files.push({ name: "ships/" + id + ".glb", data: glb });
    files.push({ name: "ships/" + id + ".ship.json", data: pretty(s.recipe) + "\n" });
  }
  const manifest = JSON.stringify({ format: "ranger-ship-pack", version: 1, forward: "+z", up: "+y", ships: entries }, null, 2);
  files.unshift({ name: "ships/ships.ts", data: "// Generated by RangerSpaceshipBuilder (ship pack). Do not edit.\nexport const PACK = " + manifest + ";\nexport const SHIPS = PACK.ships;\n" });
  files.unshift({ name: "ships/ships.json", data: manifest + "\n" });
  return files;
}

// ---- refresh -------------------------------------------------------------------

function renderShipSelect() {
  const sel = $("shipSelect");
  sel.innerHTML = "";
  const g1 = el("optgroup", { label: "Valmiit alukset" });
  for (const p of presets) g1.append(el("option", { value: "p:" + p.id, selected: current.preset && current.id === p.id }, p.recipe.name));
  sel.append(g1);
  const ids = Object.keys(mine);
  if (ids.length) {
    const g2 = el("optgroup", { label: "Omat alukset" });
    for (const id of ids) g2.append(el("option", { value: "m:" + id, selected: !current.preset && current.id === id }, mine[id].name || id));
    sel.append(g2);
  }
  if (!current.id) sel.append(el("option", { value: "", selected: true }, "(tallentamaton)"));
}

function refresh(frame) {
  if (selected >= recipe.parts.length) selected = recipe.parts.length - 1;
  renderShipSelect();
  renderShipProps();
  renderMaterials();
  renderParts();
  renderInspector();
  syncJson();
  rebuild(frame);
}

function wire() {
  $("shipSelect").onchange = (e) => {
    const v = e.target.value;
    if (v.startsWith("p:")) load(presets.find((p) => p.id === v.slice(2)));
    else if (v.startsWith("m:")) load({ id: v.slice(2), recipe: mine[v.slice(2)], preset: false });
  };
  $("btnNew").onclick = () => { current = { id: "", preset: false }; recipe = blankShip(); history = []; future = []; selected = 0; refresh(true); };
  $("btnSave").onclick = () => {
    let id = current.preset || !current.id ? slug(recipe.name) : current.id;
    if (current.preset && presets.some((p) => p.id === id)) id = id + "-oma";
    mine[id] = clone(recipe);
    localStorage.setItem("ships.mine", JSON.stringify(mine));
    current = { id, preset: false };
    renderShipSelect();
  };
  $("btnDelete").onclick = () => {
    if (current.preset || !mine[current.id]) return;
    if (!confirm(`Poistetaanko "${recipe.name}" omista aluksista?`)) return;
    delete mine[current.id];
    localStorage.setItem("ships.mine", JSON.stringify(mine));
    load(presets[0]);
  };
  $("btnUndo").onclick = undo;
  $("btnRedo").onclick = redo;
  $("chkLineup").onchange = () => rebuild(true);
  const lights = $("lightSelect");
  for (const k in LIGHTS) lights.append(el("option", { value: k }, LIGHTS[k].label));
  lights.onchange = () => view.setLights(lights.value);
  $("chkSpin").onchange = (e) => { view.lightSpin = e.target.checked; };
  $("chkTex").onchange = (e) => { view.textures = e.target.checked; view.dirty = true; };
  $("btnRecipe").onclick = () => download(slug(recipe.name) + ".ship.json", pretty(recipe) + "\n", "application/json");
  $("btnGlb").onclick = () => download(slug(recipe.name) + ".glb", glbOf(recipe, true), "model/gltf-binary");
  $("btnPack").onclick = () => {
    const ships = allShips().map((s) => ({ id: s.preset ? s.id : slug(s.id), recipe: s.id === current.id ? recipe : s.recipe }));
    download("ships.zip", zip(packFiles(ships)));
  };
  $("btnOpen").onclick = () => $("fileInput").click();
  $("fileInput").onchange = async (e) => {
    const f = e.target.files[0];
    if (!f) return;
    try {
      const r = JSON.parse(await f.text());
      current = { id: "", preset: false };
      recipe = r; history = []; future = []; selected = -1;
      refresh(true);
    } catch (err) { alert("Tiedostoa ei voitu lukea: " + err.message); }
    e.target.value = "";
  };
  const add = $("addType");
  for (const k in TYPES) add.append(el("option", { value: k }, TYPES[k].label));
  $("btnAdd").onclick = () => { const r = clone(recipe); r.parts.push(newPart(add.value)); selected = r.parts.length - 1; commit(r); };
  $("btnDup").onclick = () => {
    if (selected < 0) return;
    const r = clone(recipe);
    const p = clone(r.parts[selected]);
    p.name = (p.name || "") + " 2";
    r.parts.splice(selected + 1, 0, p);
    selected++;
    commit(r);
  };
  $("btnDel").onclick = () => { if (selected < 0) return; const r = clone(recipe); r.parts.splice(selected, 1); selected = Math.min(selected, r.parts.length - 1); commit(r); };
  const move = (d) => {
    const j = selected + d;
    if (selected < 0 || j < 0 || j >= recipe.parts.length) return;
    const r = clone(recipe);
    [r.parts[selected], r.parts[j]] = [r.parts[j], r.parts[selected]];
    selected = j;
    commit(r);
  };
  $("btnUp").onclick = () => move(-1);
  $("btnDown").onclick = () => move(1);
  for (const b of document.querySelectorAll(".tabs button")) {
    b.onclick = () => {
      for (const o of document.querySelectorAll(".tabs button")) o.classList.toggle("on", o === b);
      for (const t of document.querySelectorAll(".tab")) t.hidden = t.id !== "tab-" + b.dataset.tab;
      syncJson();
    };
  }
  $("btnApplyJson").onclick = () => {
    try {
      const r = JSON.parse($("jsonText").value);
      $("jsonError").textContent = "";
      commit(r);
    } catch (err) { $("jsonError").textContent = err.message; }
  };
  window.addEventListener("keydown", (e) => {
    if (e.target.tagName === "INPUT" || e.target.tagName === "TEXTAREA") return;
    if ((e.ctrlKey || e.metaKey) && e.key === "z") { e.preventDefault(); e.shiftKey ? redo() : undo(); }
    if ((e.ctrlKey || e.metaKey) && e.key === "y") { e.preventDefault(); redo(); }
    if (e.key === "Delete" && selected >= 0) $("btnDel").click();
    if (e.key === "f") { if (lastBuilt) view.frame(lastBuilt.info.min, lastBuilt.info.max, true); }
  });
}

async function main() {
  view = new View3D($("view"));
  view.onPick = (i) => { if (!$("chkLineup").checked) select(i); };
  await loadWasm();
  wire();
  const want = new URLSearchParams(location.search).get("ship");
  const first = presets.find((p) => p.id === want) || presets[0];
  if (first) load(first); else { recipe = blankShip(); refresh(true); }
  window.editor = { build, glbOf, get recipe() { return recipe; }, view, select };
}

main().catch((e) => { document.body.prepend(el("pre", { class: "err" }, String(e.stack || e))); });
