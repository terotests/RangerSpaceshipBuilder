// SPDX-License-Identifier: MIT
//! The part types of a recipe. Each builds its shape in its own frame
//! (+z forward, +y up, +x to the side), from the part's JSON parameters
//! with a default for every one; `recipe.rs` places it in the ship.
//!
//! | type | what |
//! | --- | --- |
//! | `hull` | a loft through cross-sections `[z, width, height, roundness, dy]`, boxy (roundness 4+) to round (2) to diamond (1); noise and ribs make it organic; greebles and windows on its skin |
//! | `wing` | a tapered, swept wing with thickness and dihedral (mirror it) |
//! | `engine` | a nacelle with a nozzle and a glowing exhaust |
//! | `cockpit` | a canopy: a half-flattened ellipsoid of glass |
//! | `box` | a box, its front tapered and lifted (wedges), corners bevelled |
//! | `cylinder` | a cylinder or cone along z |
//! | `turret` | a gun turret: base, head, barrels |
//! | `ring` | a ring round the z axis with spokes and a hub (stations) |
//! | `panel` | a solar panel of cells |
//! | `spike` | a tapering tube along a curved path (horns, claws, tendrils, antennas) |
//! | `blob` | a noisy ellipsoid (organic hulls, pods) |
//! | `dish` | a dish antenna facing +z |
//! | `truss` | a lattice beam along z |

use crate::json::Json;
use crate::math::{catmull, catmull3, clamp, spow, V3, PI};
use crate::mesh::{parse_hex, Mesh, Rgb};
use crate::noise::{fbm3, Rng};
use crate::shapes::{across, beam, circle, circle_at, loft, surface_box, tapered_box, Paint, Shade};
use ranger::prelude::*;

/// A palette entry: a name and its colour.
#[derive(Clone)]
pub struct Swatch {
    pub name: string,
    pub col: Rgb,
}

/// What a part builder reads: its parameters, the palette and its seed.
pub struct Params {
    pub json: Json,
    pub palette: Vec<Swatch>,
    pub seed: int,
}

impl Params {
    pub fn num(&self, key: &str, fallback: double) -> double {
        self.json.get(key).num_or(fallback)
    }
    pub fn int(&self, key: &str, fallback: int, lo: int, hi: int) -> int {
        let v = self.json.get(key).num_or(fallback as double).round() as int;
        if v < lo {
            lo
        } else if v > hi {
            hi
        } else {
            v
        }
    }
    pub fn flag(&self, key: &str, fallback: bool) -> bool {
        self.json.get(key).bool_or(fallback)
    }
    /// A vector: `[x, y, z]`, or one number for all three.
    pub fn v3(&self, key: &str, fallback: V3) -> V3 {
        let v = self.json.get(key);
        match v {
            Json::Num(n) => V3::new(n, n, n),
            Json::Arr(_) => V3::new(v.at_num(0, fallback.x), v.at_num(1, fallback.y), v.at_num(2, fallback.z)),
            _ => fallback,
        }
    }
    /// A colour: a palette name or "#rrggbb".
    pub fn color(&self, key: &str, fallback: &str) -> Rgb {
        let name = self.json.get(key).str_or(fallback);
        self.resolve(&name, fallback)
    }
    pub fn resolve(&self, name: &str, fallback: &str) -> Rgb {
        for s in self.palette.iter() {
            if s.name == name {
                return s.col;
            }
        }
        match parse_hex(name) {
            Some(c) => c,
            None => {
                if name != fallback {
                    return self.resolve(fallback, "#a0a0a0");
                }
                Rgb::new(0.35, 0.35, 0.35)
            }
        }
    }
    pub fn paint(&self, key: &str, fallback: &str) -> Paint {
        Paint::new(self.color(key, fallback), self.flag("glow", false))
    }
    pub fn shade(&self, fallback: double) -> Shade {
        Shade::new(self.seed + 101, self.num("panel", fallback))
    }
}

/// Builds part `p` of type `kind` into `out`; false for an unknown type.
pub fn build_part(kind: &str, p: &Params, out: &mut Mesh) -> bool {
    if kind == "hull" {
        hull(p, out);
    } else if kind == "wing" {
        wing(p, out);
    } else if kind == "engine" {
        engine(p, out);
    } else if kind == "cockpit" {
        cockpit(p, out);
    } else if kind == "box" {
        box_part(p, out);
    } else if kind == "cylinder" {
        cylinder(p, out);
    } else if kind == "turret" {
        turret(p, out);
    } else if kind == "ring" {
        ring(p, out);
    } else if kind == "panel" {
        panel(p, out);
    } else if kind == "spike" {
        spike(p, out);
    } else if kind == "blob" {
        blob(p, out);
    } else if kind == "dish" {
        dish(p, out);
    } else if kind == "truss" {
        truss(p, out);
    } else {
        return false;
    }
    true
}

// ---- hull --------------------------------------------------------------------

#[derive(Clone, Copy)]
struct Section {
    z: double,
    w: double,
    h: double,
    round: double,
    dy: double,
}

fn sections_of(p: &Params) -> Vec<Section> {
    let mut out: Vec<Section> = Vec::new();
    for s in p.json.get("sections").items().iter() {
        out.push(Section {
            z: s.at_num(0, 0.0),
            w: s.at_num(1, 1.0).max(0.0),
            h: s.at_num(2, 1.0).max(0.0),
            round: s.at_num(3, 2.0).max(0.3),
            dy: s.at_num(4, 0.0),
        });
    }
    if out.len() < 2 {
        out = vec![
            Section { z: -2.0, w: 0.8, h: 0.6, round: 2.5, dy: 0.0 },
            Section { z: 0.0, w: 1.2, h: 0.8, round: 2.5, dy: 0.0 },
            Section { z: 2.0, w: 0.05, h: 0.05, round: 2.0, dy: 0.0 },
        ];
    }
    out
}

/// The sections with `smooth` Catmull-Rom steps between each two.
fn smoothed(s: &Vec<Section>, steps: int) -> Vec<Section> {
    if steps <= 0 {
        return s.clone();
    }
    let n = s.len() as int;
    let mut out: Vec<Section> = Vec::new();
    let mut k: int = 0;
    while k < n - 1 {
        let a = s[(if k > 0 { k - 1 } else { 0 }) as usize];
        let b = s[k as usize];
        let c = s[(k + 1) as usize];
        let d = s[(if k + 2 < n { k + 2 } else { n - 1 }) as usize];
        let mut i: int = 0;
        while i <= steps {
            if i == 0 && k > 0 {
                i += 1;
                continue;
            }
            let t = (i as double) / ((steps + 1) as double);
            out.push(Section {
                z: catmull(a.z, b.z, c.z, d.z, t),
                w: catmull(a.w, b.w, c.w, d.w, t).max(0.0),
                h: catmull(a.h, b.h, c.h, d.h, t).max(0.0),
                round: catmull(a.round, b.round, c.round, d.round, t).max(0.3),
                dy: catmull(a.dy, b.dy, c.dy, d.dy, t),
            });
            i += 1;
        }
        k += 1;
    }
    out.push(s[(n - 1) as usize]);
    out
}

fn hull(p: &Params, out: &mut Mesh) {
    let given = sections_of(p);
    let steps = p.int("smooth", 0, 0, 6);
    let secs = smoothed(&given, steps);
    let sides = p.int("sides", 12, 3, 48);
    let belly = p.num("belly", 1.0);
    let noise = p.num("noise", 0.0);
    let nscale = p.num("noiseScale", 1.5);
    let ribs = p.num("ribs", 0.0);
    let rib_freq = p.num("ribFreq", 4.0);
    let seed = p.seed;
    let hull_paint = p.paint("color", "hull");
    let accent = p.paint("accent", "accent");
    let under = if p.json.has("under") { p.paint("under", "hull") } else { hull_paint };
    let mut rings: Vec<Vec<V3>> = Vec::new();
    for s in secs.iter() {
        let mut ring: Vec<V3> = Vec::new();
        let e = 2.0 / s.round;
        let mut j: int = 0;
        while j < sides {
            // start at the bottom so the seam is underneath
            let a = 2.0 * PI * (j as double) / (sides as double) - PI * 0.5;
            let mut x = spow(a.cos(), e) * s.w * 0.5;
            let mut y = spow(a.sin(), e) * s.h * 0.5;
            if y < 0.0 {
                y = y * belly;
            }
            let mut k = 1.0;
            if noise > 0.0 {
                k += noise * fbm3(x * nscale, y * nscale, s.z * nscale, seed);
            }
            if ribs > 0.0 {
                let r = (s.z * rib_freq * PI).sin();
                k += ribs * r * r;
            }
            x = x * k;
            y = y * k;
            ring.push(V3::new(x, y + s.dy, s.z));
            j += 1;
        }
        rings.push(ring);
    }
    // bands: hull colour, accent where a band starts at a section named in
    // "bands" (indices of the given sections)
    let mut accents: Vec<double> = Vec::new();
    for b in p.json.get("bands").items().iter() {
        let i = b.num_or(-1.0) as int;
        if i >= 0 && i + 1 < (given.len() as int) {
            accents.push(given[i as usize].z);
            accents.push(given[(i + 1) as usize].z);
        }
    }
    let mut bands: Vec<Paint> = Vec::new();
    let mut k: int = 0;
    while k < (secs.len() as int) - 1 {
        let zm = (secs[k as usize].z + secs[(k + 1) as usize].z) * 0.5;
        let mut paint = hull_paint;
        let mut i: int = 0;
        while i + 1 < (accents.len() as int) {
            let z0 = accents[i as usize];
            let z1 = accents[(i + 1) as usize];
            if zm >= z0.min(z1) && zm <= z0.max(z1) {
                paint = accent;
            }
            i += 2;
        }
        bands.push(paint);
        k += 1;
    }
    let mut shade = p.shade(0.12);
    let mut solid = Mesh::new();
    loft(&mut solid, &rings, &bands, hull_paint, hull_paint, false, &mut shade);
    // two-tone: the underside in its own colour
    if p.json.has("under") {
        let n = solid.count();
        let mut t: int = 0;
        while t < n {
            let c = solid.corner(t, 0).add(solid.corner(t, 1)).add(solid.corner(t, 2)).mul(1.0 / 3.0);
            let nrm = solid.normal(t);
            if nrm.y < -0.35 && c.y < 0.0 && !solid.glow[t as usize] {
                let i = (t * 3) as usize;
                let tone = solid.color(t).r / hull_paint.col.r.max(0.0001);
                let u = under.col.scale(tone.max(0.5).min(1.5));
                solid.col[i] = u.r;
                solid.col[i + 1] = u.g;
                solid.col[i + 2] = u.b;
            }
            t += 1;
        }
    }
    out.append(&solid, crate::math::Xform::identity(), 0, 0);
    // greebles and windows on the skin
    let greebles = p.int("greebles", 0, 0, 400);
    let windows = p.int("windows", 0, 0, 400);
    if greebles > 0 || windows > 0 {
        let mut rng = Rng::new(seed + 77);
        let gsize = p.num("greebleSize", 0.12);
        let garc = clamp(p.num("greebleArc", 0.5), 0.05, 1.0);
        let gpaint = p.paint("greebleColor", "dark");
        let mut g: int = 0;
        while g < greebles {
            let a = PI * 0.5 + (rng.next() - 0.5) * 2.0 * PI * garc;
            let f = rng.range(0.08, 0.92);
            let w = gsize * rng.range(0.4, 1.3);
            let l = gsize * rng.range(0.6, 2.4);
            let h = gsize * rng.range(0.15, 0.6);
            put_on_skin(out, &rings, f, a, w, l, h, gpaint, &mut shade, p.flag("greebleMirror", true));
            g += 1;
        }
        let wsize = p.num("windowSize", 0.05);
        let wpaint = Paint::new(p.color("windowColor", "glow"), true);
        let wy = p.num("windowHeight", 0.0);
        let mut w: int = 0;
        while w < windows {
            let a = wy * PI * 0.5 + (rng.next() - 0.5) * 0.25;
            let f = rng.range(0.1, 0.9);
            put_on_skin(out, &rings, f, a, wsize, wsize * 2.0, wsize * 0.2, wpaint, &mut shade, true);
            w += 1;
        }
    }
}

/// A point on the lofted skin: `f` along (0 tail .. 1 nose), angle `a`
/// (0 = +x side, PI/2 = top), and its outward normal.
fn skin_point(rings: &Vec<Vec<V3>>, f: double, a: double) -> Vec<V3> {
    let nr = rings.len() as int;
    let np = rings[0].len() as int;
    let fr = clamp(f, 0.0, 0.9999) * ((nr - 1) as double);
    let k = fr.floor() as int;
    let t = fr - (k as double);
    // the rings start at the bottom (-PI/2)
    let mut u = (a + PI * 0.5) / (2.0 * PI);
    u = u - u.floor();
    let fj = u * (np as double);
    let j = (fj.floor() as int) % np;
    let s = fj - fj.floor();
    let j1 = (j + 1) % np;
    let r0 = &rings[k as usize];
    let r1 = &rings[(k + 1) as usize];
    let p0 = r0[j as usize].lerp(r0[j1 as usize], s);
    let p1 = r1[j as usize].lerp(r1[j1 as usize], s);
    let pt = p0.lerp(p1, t);
    let along = p1.sub(p0);
    let around = r0[j1 as usize].sub(r0[j as usize]).lerp(r1[j1 as usize].sub(r1[j as usize]), t);
    let mut n = around.cross(along).norm();
    // outwards: away from the ring's middle
    let mid = V3::new(0.0, (r0[0].y + r0[(np / 2) as usize].y) * 0.5, pt.z);
    if n.dot(pt.sub(mid)) < 0.0 {
        n = n.mul(-1.0);
    }
    vec![pt, n, along]
}

fn put_on_skin(out: &mut Mesh, rings: &Vec<Vec<V3>>, f: double, a: double, w: double, l: double, h: double, paint: Paint, shade: &mut Shade, mirror: bool) {
    let s = skin_point(rings, f, a);
    surface_box(out, s[0], s[1], s[2], w, l, h, paint, shade);
    if mirror {
        let m = skin_point(rings, f, PI - a);
        surface_box(out, m[0], m[1], m[2], w, l, h, paint, shade);
    }
}

// ---- wing --------------------------------------------------------------------

fn airfoil(x: double, y: double, le: double, chord: double, thick: double) -> Vec<V3> {
    let t = thick * 0.5;
    vec![
        V3::new(x, y, le),
        V3::new(x, y + t, le - chord * 0.3),
        V3::new(x, y + t * 0.6, le - chord * 0.75),
        V3::new(x, y, le - chord),
        V3::new(x, y - t * 0.6, le - chord * 0.75),
        V3::new(x, y - t, le - chord * 0.3),
    ]
}

fn wing(p: &Params, out: &mut Mesh) {
    let span = p.num("span", 2.0);
    let root = p.num("root", 1.5);
    let tip = p.num("tip", 0.5);
    let sweep = p.num("sweep", 0.6);
    let thick = p.num("thick", 0.12);
    let tip_thick = p.num("tipThick", 0.5);
    let dihedral = p.num("dihedral", 0.0) * PI / 180.0;
    let tip_band = clamp(p.num("tipBand", 0.0), 0.0, 0.95);
    let paint = p.paint("color", "hull");
    let accent = p.paint("accent", "accent");
    let le0 = root * 0.5;
    let mut rings: Vec<Vec<V3>> = Vec::new();
    let mut bands: Vec<Paint> = Vec::new();
    let mut fs: Vec<double> = vec![0.0];
    if tip_band > 0.0 {
        fs.push(1.0 - tip_band);
    }
    fs.push(1.0);
    for (i, f) in fs.iter().enumerate() {
        let x = span * f * dihedral.cos();
        let y = span * f * dihedral.sin();
        let chord = root + (tip - root) * f;
        let le = le0 - sweep * f;
        let th = thick * (1.0 + (tip_thick - 1.0) * f);
        rings.push(airfoil(x, y, le, chord, th));
        if i > 0 {
            if tip_band > 0.0 && i == fs.len() - 1 {
                bands.push(accent);
            } else {
                bands.push(paint);
            }
        }
    }
    let mut shade = p.shade(0.08);
    let cap1 = if tip_band > 0.0 { accent } else { paint };
    loft(out, &rings, &bands, paint, cap1, false, &mut shade);
}

// ---- engine ------------------------------------------------------------------

fn engine(p: &Params, out: &mut Mesh) {
    let r = p.num("r", 0.3);
    let len = p.num("len", 1.2);
    let sides = p.int("sides", 12, 4, 32);
    let nozzle = p.num("nozzle", 1.15);
    let noz_len = p.num("nozzleLen", 0.25) * len;
    let intake = p.num("intake", 0.7);
    let body = p.paint("color", "dark");
    let ring_paint = p.paint("accent", "hull");
    let glow = Paint::new(p.color("glowColor", "glow"), true);
    let z0 = -len * 0.5;
    let z1 = len * 0.5;
    let rn = r * nozzle;
    let rings = vec![
        circle(sides, rn * 0.78, rn * 0.78, z0 + noz_len * 0.5),
        circle(sides, rn, rn, z0),
        circle(sides, r * 0.86, r * 0.86, z0 + noz_len),
        circle(sides, r, r, z0 + noz_len + 0.02 * len),
        circle(sides, r, r, z0 + noz_len + 0.12 * len),
        circle(sides, r, r, z1 - 0.12 * len),
        circle(sides, r * intake, r * intake, z1),
    ];
    let bands = vec![body, ring_paint, body, ring_paint, body, ring_paint];
    let mut shade = p.shade(0.06);
    loft(out, &rings, &bands, glow, body, false, &mut shade);
}

// ---- cockpit -------------------------------------------------------------------

fn cockpit(p: &Params, out: &mut Mesh) {
    let s = p.v3("size", V3::new(0.5, 0.35, 1.0));
    let n = p.int("detail", 10, 4, 24);
    let flat = p.num("flat", 0.25);
    let paint = p.paint("color", "glass");
    let mut rings: Vec<Vec<V3>> = Vec::new();
    let steps = (n / 2).max(3);
    let mut k: int = 0;
    while k <= steps {
        let th = PI * (k as double) / (steps as double);
        let z = -th.cos() * s.z * 0.5;
        let rr = th.sin();
        let mut ring = circle(n, rr * s.x * 0.5, rr * s.y * 0.5, z);
        for q in ring.iter_mut() {
            if q.y < 0.0 {
                q.y = q.y * flat;
            }
        }
        rings.push(ring);
        k += 1;
    }
    let bands = vec![paint];
    let mut shade = p.shade(0.0);
    loft(out, &rings, &bands, paint, paint, false, &mut shade);
}

// ---- box, cylinder -------------------------------------------------------------

fn box_part(p: &Params, out: &mut Mesh) {
    let s = p.v3("size", V3::new(1.0, 1.0, 1.0));
    let taper = p.json.get("taper");
    let tx = taper.at_num(0, taper.num_or(1.0));
    let ty = taper.at_num(1, taper.num_or(1.0));
    let mut shade = p.shade(0.1);
    tapered_box(out, s, tx, ty, p.num("lift", 0.0), p.num("bevel", 0.0), p.paint("color", "hull"), &mut shade);
}

fn cylinder(p: &Params, out: &mut Mesh) {
    let r = p.num("r", 0.5);
    let r2 = p.num("r2", r);
    let len = p.num("len", 1.0);
    let sides = p.int("sides", 12, 3, 48);
    let paint = p.paint("color", "hull");
    let caps = p.paint("capColor", "hull");
    let cap_paint = if p.json.has("capColor") { caps } else { paint };
    let rings = vec![circle(sides, r, r, -len * 0.5), circle(sides, r2, r2, len * 0.5)];
    let bands = vec![paint];
    let mut shade = p.shade(0.06);
    loft(out, &rings, &bands, cap_paint, cap_paint, false, &mut shade);
}

// ---- turret --------------------------------------------------------------------

fn turret(p: &Params, out: &mut Mesh) {
    let r = p.num("r", 0.3);
    let barrels = p.int("barrels", 2, 1, 6);
    let blen = p.num("barrelLen", 0.8);
    let br = p.num("barrelR", 0.035);
    let paint = p.paint("color", "hull");
    let dark = p.paint("accent", "dark");
    let mut shade = p.shade(0.08);
    // the base: a short cylinder standing on y = 0
    let base = vec![circle(10, r, r, 0.0), circle(10, r * 0.9, r * 0.9, r * 0.3)];
    let mut tmp = Mesh::new();
    let bands = vec![dark];
    loft(&mut tmp, &base, &bands, dark, dark, false, &mut shade);
    // the cylinder was built along z: stand it up
    let up = crate::math::Xform::new(crate::math::M3::rot_x(-90.0), V3::zero());
    out.append(&tmp, up, 0, 0);
    // the head
    let head = Mesh::new();
    let mut hm = head;
    tapered_box(&mut hm, V3::new(r * 1.4, r * 0.6, r * 1.6), 0.8, 0.7, 0.0, 0.15, paint, &mut shade);
    out.append(&hm, crate::math::Xform::new(crate::math::M3::identity(), V3::new(0.0, r * 0.55, 0.0)), 0, 0);
    // the barrels
    let gap = r * 0.35;
    let mut b: int = 0;
    while b < barrels {
        let x = (b as double - (barrels as double - 1.0) * 0.5) * gap;
        let rings = vec![circle(6, br, br, r * 0.6), circle(6, br, br, r * 0.6 + blen), circle(6, br * 1.5, br * 1.5, r * 0.6 + blen + 0.001), circle(6, br * 1.5, br * 1.5, r * 0.6 + blen + br * 4.0)];
        let mut bm = Mesh::new();
        let bb = vec![dark, dark, dark];
        loft(&mut bm, &rings, &bb, dark, dark, false, &mut shade);
        out.append(&bm, crate::math::Xform::new(crate::math::M3::identity(), V3::new(x, r * 0.55, 0.0)), 0, 0);
        b += 1;
    }
}

// ---- ring --------------------------------------------------------------------

fn ring(p: &Params, out: &mut Mesh) {
    let big_r = p.num("r", 4.0);
    let thick = p.num("thick", 0.5);
    let width = p.num("width", 0.8);
    let segs = p.int("segments", 24, 6, 96);
    let spokes = p.int("spokes", 4, 0, 16);
    let hub_r = p.num("hub", 0.6);
    let paint = p.paint("color", "hull");
    let accent = p.paint("accent", "accent");
    let mut shade = p.shade(0.1);
    // the rim: a rectangle swept round the z axis
    let mut rings: Vec<Vec<V3>> = Vec::new();
    let mut k: int = 0;
    while k < segs {
        let a = 2.0 * PI * (k as double) / (segs as double);
        let c = a.cos();
        let s = a.sin();
        let ri = big_r - thick * 0.5;
        let ro = big_r + thick * 0.5;
        let hw = width * 0.5;
        rings.push(vec![
            V3::new(c * ri, s * ri, -hw),
            V3::new(c * ro, s * ro, -hw),
            V3::new(c * ro, s * ro, hw),
            V3::new(c * ri, s * ri, hw),
        ]);
        k += 1;
    }
    let mut bands: Vec<Paint> = Vec::new();
    let windows = p.flag("windows", true);
    k = 0;
    while k < segs {
        if windows && k % 2 == 1 {
            bands.push(accent);
        } else {
            bands.push(paint);
        }
        k += 1;
    }
    loft(out, &rings, &bands, paint, paint, true, &mut shade);
    // spokes and hub
    let sw = p.num("spokeWidth", thick * 0.35);
    let mut s: int = 0;
    while s < spokes {
        let a = 2.0 * PI * (s as double) / (spokes as double) + PI / (segs as double);
        let d = V3::new(a.cos(), a.sin(), 0.0);
        beam(out, d.mul(hub_r * 0.9), d.mul(big_r - thick * 0.45), sw, paint, &mut shade);
        s += 1;
    }
    if hub_r > 0.0 {
        let hub = vec![
            circle(12, hub_r * 0.6, hub_r * 0.6, -width * 1.2),
            circle(12, hub_r, hub_r, -width * 0.6),
            circle(12, hub_r, hub_r, width * 0.6),
            circle(12, hub_r * 0.6, hub_r * 0.6, width * 1.2),
        ];
        let hb = vec![paint, accent, paint];
        loft(out, &hub, &hb, paint, paint, false, &mut shade);
    }
}

// ---- panel -------------------------------------------------------------------

fn panel(p: &Params, out: &mut Mesh) {
    let size = p.json.get("size");
    let w = size.at_num(0, 2.0);
    let l = size.at_num(1, 1.0);
    let cells = p.json.get("cells");
    let nx = (cells.at_num(0, 4.0) as int).max(1).min(24);
    let nz = (cells.at_num(1, 2.0) as int).max(1).min(24);
    let thick = p.num("thick", 0.03);
    let gap = p.num("gap", 0.04);
    let cell = p.paint("color", "solar");
    let frame = p.paint("accent", "hull");
    let mut shade = p.shade(0.15);
    let cw = (w - gap * ((nx + 1) as double)) / (nx as double);
    let cl = (l - gap * ((nz + 1) as double)) / (nz as double);
    let mut i: int = 0;
    while i < nx {
        let mut k: int = 0;
        while k < nz {
            let x = -w * 0.5 + gap + (cw + gap) * (i as double) + cw * 0.5;
            let z = -l * 0.5 + gap + (cl + gap) * (k as double) + cl * 0.5;
            let mut c = Mesh::new();
            tapered_box(&mut c, V3::new(cw, thick, cl), 1.0, 1.0, 0.0, 0.0, cell, &mut shade);
            out.append(&c, crate::math::Xform::new(crate::math::M3::identity(), V3::new(x, 0.0, z)), 0, 0);
            k += 1;
        }
        i += 1;
    }
    // the frame round it and a spine along x
    let ft = thick * 1.4;
    let hw = w * 0.5;
    let hl = l * 0.5;
    beam(out, V3::new(-hw, 0.0, -hl), V3::new(hw, 0.0, -hl), ft, frame, &mut shade);
    beam(out, V3::new(-hw, 0.0, hl), V3::new(hw, 0.0, hl), ft, frame, &mut shade);
    beam(out, V3::new(-hw, 0.0, -hl), V3::new(-hw, 0.0, hl), ft, frame, &mut shade);
    beam(out, V3::new(hw, 0.0, -hl), V3::new(hw, 0.0, hl), ft, frame, &mut shade);
    if p.flag("spine", true) {
        beam(out, V3::new(-hw - l * 0.3, 0.0, 0.0), V3::new(hw, 0.0, 0.0), ft * 1.5, frame, &mut shade);
    }
}

// ---- spike ---------------------------------------------------------------------

fn path_of(p: &Params) -> Vec<V3> {
    let mut pts: Vec<V3> = Vec::new();
    for q in p.json.get("path").items().iter() {
        pts.push(V3::new(q.at_num(0, 0.0), q.at_num(1, 0.0), q.at_num(2, 0.0)));
    }
    if pts.len() < 2 {
        pts = vec![V3::new(0.0, 0.0, 0.0), V3::new(0.0, 0.3, 0.6), V3::new(0.0, 0.4, 1.4)];
    }
    pts
}

fn spike(p: &Params, out: &mut Mesh) {
    let pts = path_of(p);
    let r0 = p.num("r0", 0.15);
    let r1 = p.num("r1", 0.0);
    let sides = p.int("sides", 6, 3, 24);
    let steps = p.int("steps", 4, 1, 16);
    let wobble = p.num("wobble", 0.0);
    let tip_frac = clamp(p.num("tip", 0.0), 0.0, 1.0);
    let paint = p.paint("color", "hull");
    let accent = p.paint("accent", "accent");
    let np = pts.len() as int;
    // the curve through the points
    let mut curve: Vec<V3> = Vec::new();
    let mut k: int = 0;
    while k < np - 1 {
        let a = pts[(if k > 0 { k - 1 } else { 0 }) as usize];
        let b = pts[k as usize];
        let c = pts[(k + 1) as usize];
        let d = pts[(if k + 2 < np { k + 2 } else { np - 1 }) as usize];
        let mut i: int = 0;
        while i < steps {
            curve.push(catmull3(a, b, c, d, (i as double) / (steps as double)));
            i += 1;
        }
        k += 1;
    }
    curve.push(pts[(np - 1) as usize]);
    let nc = curve.len() as int;
    let mut rings: Vec<Vec<V3>> = Vec::new();
    let mut bands: Vec<Paint> = Vec::new();
    let mut side = across(curve[1].sub(curve[0]));
    let mut i: int = 0;
    while i < nc {
        let prev = curve[(if i > 0 { i - 1 } else { 0 }) as usize];
        let next = curve[(if i + 1 < nc { i + 1 } else { nc - 1 }) as usize];
        let dir = next.sub(prev).norm();
        // carry the side along the curve (parallel transport)
        side = side.sub(dir.mul(side.dot(dir)));
        if side.len() < 1.0e-6 {
            side = across(dir);
        }
        side = side.norm();
        let f = (i as double) / ((nc - 1) as double);
        let mut r = r0 + (r1 - r0) * f;
        if wobble > 0.0 {
            r = r * (1.0 + wobble * (f * PI * 6.0).sin());
        }
        rings.push(circle_at(sides, r.max(0.0005), curve[i as usize], side, dir));
        if i > 0 {
            if tip_frac > 0.0 && f > 1.0 - tip_frac {
                bands.push(accent);
            } else {
                bands.push(paint);
            }
        }
        i += 1;
    }
    let mut shade = p.shade(0.05);
    let cap1 = bands[bands.len() - 1];
    loft(out, &rings, &bands, paint, cap1, false, &mut shade);
}

// ---- blob --------------------------------------------------------------------

fn blob(p: &Params, out: &mut Mesh) {
    let s = p.v3("size", V3::new(1.0, 0.7, 1.6));
    let lat = p.int("rings", 8, 3, 32);
    let segs = p.int("sides", 12, 4, 48);
    let noise = p.num("noise", 0.15);
    let freq = p.num("noiseScale", 1.2);
    let ribs = p.num("ribs", 0.0);
    let rib_freq = p.num("ribFreq", 4.0);
    let paint = p.paint("color", "hull");
    let seed = p.seed;
    let mut rings: Vec<Vec<V3>> = Vec::new();
    let mut k: int = 0;
    while k <= lat {
        let th = PI * (k as double) / (lat as double);
        let z = -th.cos() * s.z * 0.5;
        let rr = th.sin();
        let mut ring: Vec<V3> = Vec::new();
        let mut j: int = 0;
        while j < segs {
            let a = 2.0 * PI * (j as double) / (segs as double) - PI * 0.5;
            let mut q = V3::new(a.cos() * rr * s.x * 0.5, a.sin() * rr * s.y * 0.5, z);
            let mut m = 1.0;
            if noise > 0.0 {
                m += noise * fbm3(q.x * freq + 5.0, q.y * freq, q.z * freq, seed);
            }
            if ribs > 0.0 {
                let r = (z * rib_freq * PI).sin();
                m += ribs * r * r;
            }
            q = V3::new(q.x * m, q.y * m, q.z);
            ring.push(q);
            j += 1;
        }
        rings.push(ring);
        k += 1;
    }
    let bands = vec![paint];
    let mut shade = p.shade(0.06);
    loft(out, &rings, &bands, paint, paint, false, &mut shade);
}

// ---- dish --------------------------------------------------------------------

fn dish(p: &Params, out: &mut Mesh) {
    let r = p.num("r", 0.8);
    let depth = p.num("depth", 0.25) * r;
    let sides = p.int("sides", 16, 6, 48);
    let paint = p.paint("color", "hull");
    let back = p.paint("accent", "dark");
    let t = r * 0.05;
    let rings = vec![
        circle(sides, 0.001, 0.001, 0.0),
        circle(sides, r * 0.5, r * 0.5, depth * 0.25),
        circle(sides, r, r, depth),
        circle(sides, r * 1.02, r * 1.02, depth - t),
        circle(sides, r * 0.5, r * 0.5, depth * 0.25 - t * 1.5),
        circle(sides, 0.001, 0.001, -t * 2.0),
    ];
    let bands = vec![paint, paint, back, back, back];
    let mut shade = p.shade(0.04);
    loft(out, &rings, &bands, paint, back, false, &mut shade);
    // the feed in the middle
    beam(out, V3::new(0.0, 0.0, 0.0), V3::new(0.0, 0.0, depth * 1.6), r * 0.05, back, &mut shade);
}

// ---- truss -------------------------------------------------------------------

fn truss(p: &Params, out: &mut Mesh) {
    let len = p.num("len", 4.0);
    let w = p.num("width", 0.4);
    let n = p.int("segments", 6, 1, 64);
    let t = p.num("thick", 0.04);
    let paint = p.paint("color", "hull");
    let mut shade = p.shade(0.08);
    let h = w * 0.5;
    let z0 = -len * 0.5;
    let corners = vec![V3::new(h, h, 0.0), V3::new(-h, h, 0.0), V3::new(-h, -h, 0.0), V3::new(h, -h, 0.0)];
    for c in corners.iter() {
        beam(out, V3::new(c.x, c.y, z0), V3::new(c.x, c.y, z0 + len), t, paint, &mut shade);
    }
    let step = len / (n as double);
    let mut i: int = 0;
    while i <= n {
        let z = z0 + step * (i as double);
        let mut k: int = 0;
        while k < 4 {
            let a = corners[k as usize];
            let b = corners[((k + 1) % 4) as usize];
            beam(out, V3::new(a.x, a.y, z), V3::new(b.x, b.y, z), t * 0.7, paint, &mut shade);
            if i < n {
                // diagonals, alternating
                let (pa, pb) = if i % 2 == 0 { (a, b) } else { (b, a) };
                beam(out, V3::new(pa.x, pa.y, z), V3::new(pb.x, pb.y, z + step), t * 0.6, paint, &mut shade);
            }
            k += 1;
        }
        i += 1;
    }
}
