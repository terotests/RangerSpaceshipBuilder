// SPDX-License-Identifier: MIT
//! Procedural surface textures that tile seamlessly: for each material a
//! brightness map (multiplied with the part's colour), a metal/roughness
//! map (glTF's layout: G roughness, B metal) and a normal map made from a
//! height field, so panel seams, scales and veins catch the light.
//!
//! Every pattern works in a unit tile (u, v in 0..1) and only uses noise
//! and cells that wrap at its edges.

use crate::material::Material;
use crate::math::clamp;
use crate::noise::{hash3, Rng};
use ranger::prelude::*;

pub struct TexSet {
    pub size: int,
    /// RGB, size × size × 3 bytes each
    pub base: Vec<u8>,
    pub orm: Vec<u8>,
    pub normal: Vec<u8>,
}

/// What a pattern says about one point.
#[derive(Clone, Copy)]
struct Sample {
    h: double,
    a: double,
    rough: double,
    metal: double,
    /// 1 on a panel's edge (for wear), 0 inside
    edge: double,
}

fn smooth(t: double) -> double {
    let x = clamp(t, 0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn fract(x: double) -> double {
    x - x.floor()
}

fn wrap(i: int, n: int) -> int {
    let r = i % n;
    if r < 0 {
        r + n
    } else {
        r
    }
}

/// Value noise that repeats every `period` lattice cells.
pub fn pnoise(x: double, y: double, period: int, seed: int) -> double {
    let fx = x.floor();
    let fy = y.floor();
    let ix = fx as int;
    let iy = fy as int;
    let tx = smooth(x - fx);
    let ty = smooth(y - fy);
    let x0 = wrap(ix, period);
    let x1 = wrap(ix + 1, period);
    let y0 = wrap(iy, period);
    let y1 = wrap(iy + 1, period);
    let a = hash3(x0, y0, 0, seed);
    let b = hash3(x1, y0, 0, seed);
    let c = hash3(x0, y1, 0, seed);
    let d = hash3(x1, y1, 0, seed);
    let top = a + (b - a) * tx;
    let bot = c + (d - c) * tx;
    (top + (bot - top) * ty) * 2.0 - 1.0
}

/// Tiling fractal noise over the unit tile, starting at `base` cells.
pub fn pfbm(u: double, v: double, base: int, octaves: int, seed: int) -> double {
    let mut sum = 0.0;
    let mut amp = 0.5;
    let mut p = base;
    let mut k: int = 0;
    while k < octaves {
        sum += amp * pnoise(u * (p as double), v * (p as double), p, seed + k * 13);
        amp *= 0.5;
        p *= 2;
        k += 1;
    }
    sum
}

/// Distances to the nearest and second nearest feature point of a tiling
/// Voronoi pattern with g × g cells: [f1, f2, id of the nearest].
fn voronoi(u: double, v: double, g: int, seed: int) -> Vec<double> {
    let x = u * (g as double);
    let y = v * (g as double);
    let cx = x.floor() as int;
    let cy = y.floor() as int;
    let mut f1 = 9.0;
    let mut f2 = 9.0;
    let mut id = 0.0;
    let mut j: int = -1;
    while j <= 1 {
        let mut i: int = -1;
        while i <= 1 {
            let gx = cx + i;
            let gy = cy + j;
            let wx = wrap(gx, g);
            let wy = wrap(gy, g);
            let px = (gx as double) + hash3(wx, wy, 1, seed);
            let py = (gy as double) + hash3(wx, wy, 2, seed);
            let dx = px - x;
            let dy = py - y;
            let d = (dx * dx + dy * dy).sqrt();
            if d < f1 {
                f2 = f1;
                f1 = d;
                id = hash3(wx, wy, 3, seed);
            } else if d < f2 {
                f2 = d;
            }
            i += 1;
        }
        j += 1;
    }
    vec![f1, f2, id]
}

/// Panels: `rows` rows, each cut into 1..`maxcut`+1 panels at random places.
/// Answers [distance to the nearest seam in tile units, panel id, x in panel 0..1, y in panel 0..1].
fn panels(u: double, v: double, rows: int, maxcut: int, seed: int) -> Vec<double> {
    let row = (v * (rows as double)).floor() as int;
    let rv = fract(v * (rows as double));
    let mut rng = Rng::new(seed * 31 + row * 977 + 5);
    let n = 1 + ((rng.next() * ((maxcut + 1) as double)).floor() as int).min(maxcut);
    // cut positions, sorted, starting at a random offset so rows do not line up
    let off = rng.next();
    let mut cuts: Vec<double> = Vec::new();
    let mut k: int = 0;
    while k < n {
        let c = (k as double + 0.15 + 0.7 * rng.next()) / (n as double);
        cuts.push(fract(c + off));
        k += 1;
    }
    // sort (few items)
    let mut a: int = 0;
    while a < n {
        let mut b: int = a + 1;
        while b < n {
            if cuts[b as usize] < cuts[a as usize] {
                let t = cuts[a as usize];
                cuts[a as usize] = cuts[b as usize];
                cuts[b as usize] = t;
            }
            b += 1;
        }
        a += 1;
    }
    // which panel u falls in (wrapping)
    let mut idx: int = n - 1;
    let mut left = cuts[(n - 1) as usize] - 1.0;
    let mut right = cuts[0];
    k = 0;
    while k < n {
        let c0 = cuts[k as usize];
        let c1 = if k + 1 < n { cuts[(k + 1) as usize] } else { cuts[0] + 1.0 };
        if u >= c0 && u < c1 {
            idx = k;
            left = c0;
            right = c1;
        }
        k += 1;
    }
    if u < cuts[0] {
        idx = n - 1;
        left = cuts[(n - 1) as usize] - 1.0;
        right = cuts[0];
    }
    let du = (u - left).min(right - u);
    let dv = (rv.min(1.0 - rv)) / (rows as double);
    let id = hash3(row, idx, 7, seed);
    let px = (u - left) / (right - left).max(0.0001);
    vec![du.min(dv), id, px, rv]
}

fn sample(m: &Material, u: double, v: double) -> Sample {
    let seed = m.seed * 101 + 7;
    let p = m.pattern.as_str();
    let fine = pfbm(u, v, 32, 2, seed + 3);
    let mut s = Sample { h: 0.5, a: 0.9, rough: m.rough, metal: m.metal, edge: 0.0 };
    if p == "plates" || p == "armor" {
        let armor = p == "armor";
        let pp = if armor { panels(u, v, 2, 2, seed) } else { panels(u, v, 4, 3, seed) };
        let d = pp[0];
        let id = pp[1];
        let seam = if armor { 0.012 } else { 0.006 };
        let bevel = if armor { 0.05 } else { 0.012 };
        let groove = smooth((d - seam * 0.5) / seam);
        let lip = smooth((d - seam) / bevel);
        s.h = 0.15 + 0.35 * groove + 0.5 * lip;
        s.a = 0.82 + 0.16 * (id - 0.5) + 0.03 * fine;
        s.a = s.a * (0.45 + 0.55 * groove);
        s.rough = m.rough + 0.15 * (id - 0.5);
        s.edge = 1.0 - smooth((d - seam) / (bevel * 1.5));
        // rivets along some panels' edges, an inset hatch in others
        if !armor && id > 0.55 {
            let along = fract(pp[2] * 10.0);
            let r = ((along - 0.5) * (along - 0.5) * 0.0004 + (d - seam * 2.6) * (d - seam * 2.6)).sqrt();
            if r < 0.0028 {
                s.h += 0.25 * (1.0 - r / 0.0028);
                s.a *= 0.85;
            }
        }
        if id < 0.18 {
            let ix = (pp[2] - 0.5).abs();
            let iy = (pp[3] - 0.5).abs();
            let inset = smooth((0.32 - ix.max(iy)) * 40.0);
            s.h -= 0.15 * inset;
            s.a *= 1.0 - 0.12 * inset;
        }
        if armor {
            // scratches: thin bright lines
            let sc = pnoise(u * 64.0 + v * 7.0, v * 3.0, 64, seed + 9);
            if sc.abs() < 0.02 {
                s.a += 0.15;
                s.rough -= 0.15;
            }
        }
    } else if p == "hex" {
        // the nearer of two offset lattices of cells 1 × sqrt(3): hexagons;
        // 5 across and 3 rows of pairs up the tile repeat (and are within 4 %
        // of regular)
        let r3 = 1.7320508;
        let px = u * 5.0;
        let py = v * 3.0 * r3;
        let ax = fract(px) - 0.5;
        let ay = fract(py / r3) * r3 - r3 * 0.5;
        let bx = fract(px - 0.5) - 0.5;
        let by = fract((py - r3 * 0.5) / r3) * r3 - r3 * 0.5;
        let use_a = ax * ax + ay * ay < bx * bx + by * by;
        let cx = if use_a { ax } else { bx };
        let cy = if use_a { ay } else { by };
        let ix = if use_a { (px.floor()) as int } else { ((px - 0.5).floor()) as int };
        let iy = if use_a { ((py / r3).floor()) as int } else { (((py - r3 * 0.5) / r3).floor()) as int };
        let qx = cx.abs();
        let qy = cy.abs();
        // distance to the hexagon's edge: 0.5 at the edge (flat sides left and right)
        let hd = (qx).max(qx * 0.5 + qy * 0.8660254) / 0.5;
        let border = smooth((0.9 - hd) * 14.0);
        let id = hash3(wrap(ix, 5), wrap(iy, 3), if use_a { 5 } else { 6 }, seed);
        s.h = 0.2 + 0.55 * border + 0.25 * (1.0 - hd).max(0.0) * border;
        s.a = (0.72 + 0.22 * id) * (0.4 + 0.6 * border);
        s.edge = 1.0 - border;
        s.rough = m.rough + 0.2 * (1.0 - border);
    } else if p == "organic" {
        let vo = voronoi(u, v, 7, seed);
        let vein = smooth((vo[1] - vo[0]) * 7.0);
        let bump = 1.0 - vo[0];
        let n = pfbm(u, v, 8, 3, seed + 5);
        s.h = 0.25 + 0.45 * vein * bump + 0.2 * n;
        s.a = (0.62 + 0.3 * vo[2] + 0.15 * n) * (0.35 + 0.65 * vein);
        s.rough = m.rough + 0.25 * (1.0 - vein);
        s.metal = 0.0;
        s.edge = 1.0 - vein;
    } else if p == "carbon" {
        let gx = u * 16.0;
        let gy = v * 16.0;
        let cell = ((gx.floor() as int) + (gy.floor() as int)) % 2;
        let t = if cell == 0 { fract(gx) } else { fract(gy) };
        let ridge = (t * 3.14159265).sin();
        s.h = 0.3 + 0.4 * ridge;
        s.a = 0.55 + 0.25 * ridge + if cell == 0 { 0.05 } else { -0.05 };
        s.rough = m.rough + 0.1 * (1.0 - ridge);
    } else if p == "solar" {
        let gx = fract(u * 4.0);
        let gy = fract(v * 8.0);
        let line = smooth((gx.min(1.0 - gx).min(gy.min(1.0 - gy) * 0.5)) * 60.0);
        let fingers = fract(u * 48.0);
        let finger = smooth((fingers.min(1.0 - fingers) - 0.04) * 30.0);
        s.h = 0.4 + 0.3 * line;
        s.a = 0.55 * line * (0.85 + 0.15 * finger) + (1.0 - line) * 1.0;
        s.metal = m.metal + 0.5 * (1.0 - line);
        s.rough = m.rough + 0.3 * (1.0 - line);
    } else if p == "vents" {
        let t = fract(v * 10.0);
        let slat = smooth(t * 6.0) * (1.0 - smooth((t - 0.8) * 10.0));
        let frame = fract(u * 2.0);
        let rim = smooth((frame.min(1.0 - frame) - 0.03) * 40.0);
        s.h = 0.15 + 0.7 * slat * rim + 0.15 * (1.0 - rim);
        s.a = (0.35 + 0.6 * slat) * (0.6 + 0.4 * rim) + 0.3 * (1.0 - rim);
        s.edge = 1.0 - rim;
    } else if p == "glass" {
        let streak = pfbm(u * 0.5 + v, v, 4, 2, seed);
        s.h = 0.5;
        s.a = 0.95 + 0.05 * streak;
        s.rough = m.rough + 0.05 * streak.abs();
    } else if p == "hazard" {
        let t = fract((u + v) * 6.0);
        let band = smooth((t - 0.5).abs() * 40.0 - 9.0);
        let stripe = if t < 0.5 { 1.0 } else { 0.22 };
        s.h = 0.5 + 0.1 * band;
        s.a = stripe * (0.9 + 0.1 * fine);
    } else {
        // paint: fine grain and a few chips
        let chip = pfbm(u, v, 16, 3, seed + 11);
        s.h = 0.5 + 0.08 * fine;
        s.a = 0.9 + 0.05 * fine;
        if chip > 0.33 {
            s.a *= 0.75;
            s.h -= 0.1;
            s.metal = (s.metal + 0.5).min(1.0);
            s.rough -= 0.1;
        }
    }
    // grime: large soft blotches, streaks running down v, worn bright edges
    if m.grime > 0.0 {
        let blot = pfbm(u, v, 4, 3, seed + 21);
        let streak = pnoise(u * 32.0, v * 2.0, 32, seed + 23) * 0.5 + pnoise(u * 64.0, v * 4.0, 64, seed + 29) * 0.5;
        let dirt = clamp(blot * 1.2 + streak * 0.6, 0.0, 1.0);
        s.a *= 1.0 - m.grime * 0.45 * dirt;
        s.rough += m.grime * 0.25 * dirt;
        s.a += m.grime * 0.25 * s.edge;
    }
    s.a += 0.03 * fine;
    s
}

fn byte(x: double) -> u8 {
    let v = (clamp(x, 0.0, 1.0) * 255.0).round() as int;
    v as u8
}

/// The three maps of material `m`, `size` × `size` pixels (a power of two).
pub fn generate(m: &Material, size: int) -> TexSet {
    let n = size;
    let count = (n * n) as usize;
    let mut heights: Vec<double> = vec![0.0; count];
    let mut base = vec![0u8; count * 3];
    let mut orm = vec![0u8; count * 3];
    let mut normal = vec![0u8; count * 3];
    let mut y: int = 0;
    while y < n {
        let mut x: int = 0;
        while x < n {
            let u = ((x as double) + 0.5) / (n as double);
            let v = ((y as double) + 0.5) / (n as double);
            let s = sample(m, u, v);
            let i = (y * n + x) as usize;
            heights[i] = s.h;
            let a = byte(s.a);
            base[i * 3] = a;
            base[i * 3 + 1] = a;
            base[i * 3 + 2] = a;
            orm[i * 3] = 255;
            orm[i * 3 + 1] = byte(clamp(s.rough, 0.03, 1.0));
            orm[i * 3 + 2] = byte(s.metal);
            x += 1;
        }
        y += 1;
    }
    // normals from the height field (wrapping at the edges); +y up (glTF)
    let k = m.depth * (n as double) / 64.0;
    y = 0;
    while y < n {
        let mut x: int = 0;
        while x < n {
            let l = heights[(y * n + wrap(x - 1, n)) as usize];
            let r = heights[(y * n + wrap(x + 1, n)) as usize];
            let d = heights[(wrap(y - 1, n) * n + x) as usize];
            let up = heights[(wrap(y + 1, n) * n + x) as usize];
            let nx = -(r - l) * k;
            let ny = (up - d) * k;
            let len = (nx * nx + ny * ny + 1.0).sqrt();
            let i = ((y * n + x) * 3) as usize;
            normal[i] = byte(nx / len * 0.5 + 0.5);
            normal[i + 1] = byte(ny / len * 0.5 + 0.5);
            normal[i + 2] = byte(1.0 / len * 0.5 + 0.5);
            x += 1;
        }
        y += 1;
    }
    TexSet { size: n, base: base, orm: orm, normal: normal }
}
