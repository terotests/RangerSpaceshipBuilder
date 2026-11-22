// SPDX-License-Identifier: MIT
//! The building blocks the parts are made of: lofts (rings of points joined
//! into a closed solid), boxes and beams. A closed solid is turned to face
//! outwards by the sign of its volume, so the builders need not care which
//! way round their rings go.

use crate::math::{V3, PI};
use crate::mesh::{Mesh, Rgb};
use crate::noise::Rng;
use ranger::prelude::*;

/// The colour of one band of a loft (between ring k and k + 1) or a cap.
#[derive(Clone, Copy, Debug)]
pub struct Paint {
    pub col: Rgb,
    pub glow: bool,
}

impl Paint {
    pub fn new(col: Rgb, glow: bool) -> Paint {
        Paint { col: col, glow: glow }
    }
}

/// Panel shading: each quad a little lighter or darker, a plated look.
pub struct Shade {
    pub rng: Rng,
    pub amount: double,
}

impl Shade {
    pub fn new(seed: int, amount: double) -> Shade {
        Shade { rng: Rng::new(seed), amount: amount }
    }
    pub fn vary(&mut self, p: Paint) -> Rgb {
        if self.amount <= 0.0 || p.glow {
            return p.col;
        }
        let k = 1.0 + self.amount * (self.rng.next() - 0.5);
        p.col.scale(k)
    }
}

/// Twice the signed volume of a closed mesh times 3 (only its sign is used).
pub fn signed_volume(m: &Mesh) -> double {
    let mut v = 0.0;
    let n = m.count();
    let mut t: int = 0;
    while t < n {
        let a = m.corner(t, 0);
        let b = m.corner(t, 1);
        let c = m.corner(t, 2);
        v += a.dot(b.cross(c));
        t += 1;
    }
    v
}

/// Turns every triangle round.
pub fn flip(m: &mut Mesh) {
    let n = m.count();
    let mut t: int = 0;
    while t < n {
        let i = (t * 9) as usize;
        let bx = m.pos[i + 3];
        let by = m.pos[i + 4];
        let bz = m.pos[i + 5];
        m.pos[i + 3] = m.pos[i + 6];
        m.pos[i + 4] = m.pos[i + 7];
        m.pos[i + 5] = m.pos[i + 8];
        m.pos[i + 6] = bx;
        m.pos[i + 7] = by;
        m.pos[i + 8] = bz;
        t += 1;
    }
}

/// Adds the closed solid `solid` to `out`, facing outwards.
pub fn add_solid(out: &mut Mesh, solid: &mut Mesh) {
    if signed_volume(solid) < 0.0 {
        flip(solid);
    }
    let n = solid.count();
    let mut t: int = 0;
    while t < n {
        let a = solid.corner(t, 0);
        let b = solid.corner(t, 1);
        let c = solid.corner(t, 2);
        out.tri(a, b, c, solid.color(t), solid.glow[t as usize]);
        t += 1;
    }
}

fn center(ring: &Vec<V3>) -> V3 {
    let mut c = V3::zero();
    for p in ring.iter() {
        c = c.add(*p);
    }
    c.mul(1.0 / (ring.len() as double))
}

/// Joins `rings` (each the same number of points) into a solid: band k
/// between ring k and k + 1 gets `bands[k]`. `closed` joins the last ring
/// to the first (a torus) and leaves the caps out; otherwise the first and
/// last rings are capped with `cap0` / `cap1` (fans from the ring's middle).
pub fn loft(out: &mut Mesh, rings: &Vec<Vec<V3>>, bands: &Vec<Paint>, cap0: Paint, cap1: Paint, closed: bool, shade: &mut Shade) {
    let mut solid = Mesh::new();
    let nr = rings.len() as int;
    if nr < 2 {
        return;
    }
    let np = rings[0].len() as int;
    let last_band = if closed { nr } else { nr - 1 };
    let mut k: int = 0;
    while k < last_band {
        let r0 = &rings[k as usize];
        let r1 = &rings[((k + 1) % nr) as usize];
        let paint = if (k as usize) < bands.len() { bands[k as usize] } else { bands[bands.len() - 1] };
        let mut j: int = 0;
        while j < np {
            let j1 = (j + 1) % np;
            let col = shade.vary(paint);
            solid.quad(r0[j as usize], r0[j1 as usize], r1[j1 as usize], r1[j as usize], col, paint.glow);
            j += 1;
        }
        k += 1;
    }
    if !closed {
        let first = &rings[0];
        let c0 = center(first);
        let mut j: int = 0;
        while j < np {
            let j1 = (j + 1) % np;
            solid.tri(c0, first[j1 as usize], first[j as usize], cap0.col, cap0.glow);
            j += 1;
        }
        let lastr = &rings[(nr - 1) as usize];
        let c1 = center(lastr);
        j = 0;
        while j < np {
            let j1 = (j + 1) % np;
            solid.tri(c1, lastr[j as usize], lastr[j1 as usize], cap1.col, cap1.glow);
            j += 1;
        }
    }
    add_solid(out, &mut solid);
}

/// A ring of `n` points round the z axis at height z: an ellipse of radii
/// rx, ry (the first point at +x, counter-clockwise seen from +z).
pub fn circle(n: int, rx: double, ry: double, z: double) -> Vec<V3> {
    let mut ring: Vec<V3> = Vec::new();
    let mut j: int = 0;
    while j < n {
        let a = 2.0 * PI * (j as double) / (n as double);
        ring.push(V3::new(a.cos() * rx, a.sin() * ry, z));
        j += 1;
    }
    ring
}

/// A ring placed at `c` across the direction `dir`, its first point along
/// `side` (which must be across `dir`).
pub fn circle_at(n: int, r: double, c: V3, side: V3, dir: V3) -> Vec<V3> {
    let nn = side.norm();
    let b = dir.norm().cross(nn);
    let mut ring: Vec<V3> = Vec::new();
    let mut j: int = 0;
    while j < n {
        let a = 2.0 * PI * (j as double) / (n as double);
        ring.push(c.add(nn.mul(a.cos() * r)).add(b.mul(a.sin() * r)));
        j += 1;
    }
    ring
}

/// Some direction across `d`.
pub fn across(d: V3) -> V3 {
    let up = if d.norm().y.abs() > 0.9 { V3::new(1.0, 0.0, 0.0) } else { V3::new(0.0, 1.0, 0.0) };
    d.cross(up).norm()
}

/// A box of size `s` centred at the origin; the front (+z) face scaled by
/// `tx`, `ty` and lifted by `lift`; corners cut by `bevel` (0 = sharp).
pub fn tapered_box(out: &mut Mesh, s: V3, tx: double, ty: double, lift: double, bevel: double, paint: Paint, shade: &mut Shade) {
    let hx = s.x * 0.5;
    let hy = s.y * 0.5;
    let hz = s.z * 0.5;
    let mut back: Vec<V3> = Vec::new();
    let mut front: Vec<V3> = Vec::new();
    let bv = crate::math::clamp(bevel, 0.0, 0.49) * if hx < hy { hx * 2.0 } else { hy * 2.0 };
    let mut corners: Vec<V3> = Vec::new();
    if bv <= 0.0 {
        corners.push(V3::new(hx, -hy, 0.0));
        corners.push(V3::new(hx, hy, 0.0));
        corners.push(V3::new(-hx, hy, 0.0));
        corners.push(V3::new(-hx, -hy, 0.0));
    } else {
        corners.push(V3::new(hx, -hy + bv, 0.0));
        corners.push(V3::new(hx, hy - bv, 0.0));
        corners.push(V3::new(hx - bv, hy, 0.0));
        corners.push(V3::new(-hx + bv, hy, 0.0));
        corners.push(V3::new(-hx, hy - bv, 0.0));
        corners.push(V3::new(-hx, -hy + bv, 0.0));
        corners.push(V3::new(-hx + bv, -hy, 0.0));
        corners.push(V3::new(hx - bv, -hy, 0.0));
    }
    for c in corners.iter() {
        back.push(V3::new(c.x, c.y, -hz));
        front.push(V3::new(c.x * tx, c.y * ty + lift, hz));
    }
    let rings = vec![back, front];
    let bands = vec![paint];
    loft(out, &rings, &bands, paint, paint, false, shade);
}

/// A square beam from `a` to `b`, `t` thick.
pub fn beam(out: &mut Mesh, a: V3, b: V3, t: double, paint: Paint, shade: &mut Shade) {
    let d = b.sub(a);
    if d.len() < 1.0e-9 {
        return;
    }
    let side = across(d);
    let r = t * 0.7071;
    let ring0 = circle_at(4, r, a, side, d);
    let ring1 = circle_at(4, r, b, side, d);
    let rings = vec![ring0, ring1];
    let bands = vec![paint];
    loft(out, &rings, &bands, paint, paint, false, shade);
}

/// A small box sitting on a surface at `p` with normal `n`: `w` across,
/// `l` along `along`, `h` high.
pub fn surface_box(out: &mut Mesh, p: V3, n: V3, along: V3, w: double, l: double, h: double, paint: Paint, shade: &mut Shade) {
    let up = n.norm();
    let mut fwd = along.sub(up.mul(along.dot(up)));
    if fwd.len() < 1.0e-6 {
        fwd = across(up);
    }
    let f = fwd.norm();
    let s = up.cross(f);
    let mut solid = Mesh::new();
    let hw = w * 0.5;
    let hl = l * 0.5;
    // sunk a little into the surface so it never floats
    let base = p.sub(up.mul(h * 0.3));
    let top = p.add(up.mul(h));
    let c = |o: V3, x: double, z: double| -> V3 { o.add(s.mul(x)).add(f.mul(z)) };
    let b0 = c(base, -hw, -hl);
    let b1 = c(base, hw, -hl);
    let b2 = c(base, hw, hl);
    let b3 = c(base, -hw, hl);
    let t0 = c(top, -hw, -hl);
    let t1 = c(top, hw, -hl);
    let t2 = c(top, hw, hl);
    let t3 = c(top, -hw, hl);
    let col = shade.vary(paint);
    solid.quad(b0, b1, b2, b3, col, paint.glow);
    solid.quad(t0, t3, t2, t1, col, paint.glow);
    solid.quad(b0, t0, t1, b1, col, paint.glow);
    solid.quad(b1, t1, t2, b2, col, paint.glow);
    solid.quad(b2, t2, t3, b3, col, paint.glow);
    solid.quad(b3, t3, t0, b0, col, paint.glow);
    add_solid(out, &mut solid);
}
