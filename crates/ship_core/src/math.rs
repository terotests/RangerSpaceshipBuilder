// SPDX-License-Identifier: MIT
//! Vectors, 3 × 3 matrices and the placement of a part in the ship.

use ranger::prelude::*;

pub const PI: double = 3.141592653589793;

#[derive(Clone, Copy, Debug)]
pub struct V3 {
    pub x: double,
    pub y: double,
    pub z: double,
}

impl V3 {
    pub fn new(x: double, y: double, z: double) -> V3 {
        V3 { x: x, y: y, z: z }
    }
    pub fn zero() -> V3 {
        V3::new(0.0, 0.0, 0.0)
    }
    pub fn add(&self, o: V3) -> V3 {
        V3::new(self.x + o.x, self.y + o.y, self.z + o.z)
    }
    pub fn sub(&self, o: V3) -> V3 {
        V3::new(self.x - o.x, self.y - o.y, self.z - o.z)
    }
    pub fn mul(&self, k: double) -> V3 {
        V3::new(self.x * k, self.y * k, self.z * k)
    }
    pub fn mul3(&self, o: V3) -> V3 {
        V3::new(self.x * o.x, self.y * o.y, self.z * o.z)
    }
    pub fn dot(&self, o: V3) -> double {
        self.x * o.x + self.y * o.y + self.z * o.z
    }
    pub fn cross(&self, o: V3) -> V3 {
        V3::new(self.y * o.z - self.z * o.y, self.z * o.x - self.x * o.z, self.x * o.y - self.y * o.x)
    }
    pub fn len(&self) -> double {
        self.dot(*self).sqrt()
    }
    pub fn norm(&self) -> V3 {
        let l = self.len();
        if l < 1.0e-12 {
            return V3::new(0.0, 0.0, 1.0);
        }
        self.mul(1.0 / l)
    }
    pub fn lerp(&self, o: V3, t: double) -> V3 {
        self.add(o.sub(*self).mul(t))
    }
}

/// A 3 × 3 matrix, row by row.
#[derive(Clone, Copy, Debug)]
pub struct M3 {
    pub a: double,
    pub b: double,
    pub c: double,
    pub d: double,
    pub e: double,
    pub f: double,
    pub g: double,
    pub h: double,
    pub i: double,
}

impl M3 {
    pub fn identity() -> M3 {
        M3 { a: 1.0, b: 0.0, c: 0.0, d: 0.0, e: 1.0, f: 0.0, g: 0.0, h: 0.0, i: 1.0 }
    }
    pub fn scale(s: V3) -> M3 {
        M3 { a: s.x, b: 0.0, c: 0.0, d: 0.0, e: s.y, f: 0.0, g: 0.0, h: 0.0, i: s.z }
    }
    pub fn rot_x(deg: double) -> M3 {
        let r = deg * PI / 180.0;
        let (s, c) = (r.sin(), r.cos());
        M3 { a: 1.0, b: 0.0, c: 0.0, d: 0.0, e: c, f: -s, g: 0.0, h: s, i: c }
    }
    pub fn rot_y(deg: double) -> M3 {
        let r = deg * PI / 180.0;
        let (s, c) = (r.sin(), r.cos());
        M3 { a: c, b: 0.0, c: s, d: 0.0, e: 1.0, f: 0.0, g: -s, h: 0.0, i: c }
    }
    pub fn rot_z(deg: double) -> M3 {
        let r = deg * PI / 180.0;
        let (s, c) = (r.sin(), r.cos());
        M3 { a: c, b: -s, c: 0.0, d: s, e: c, f: 0.0, g: 0.0, h: 0.0, i: 1.0 }
    }
    /// Turns of `r` degrees: first round x, then y, then z.
    pub fn euler(r: V3) -> M3 {
        M3::rot_z(r.z).mul(M3::rot_y(r.y)).mul(M3::rot_x(r.x))
    }
    pub fn mul(&self, o: M3) -> M3 {
        M3 {
            a: self.a * o.a + self.b * o.d + self.c * o.g,
            b: self.a * o.b + self.b * o.e + self.c * o.h,
            c: self.a * o.c + self.b * o.f + self.c * o.i,
            d: self.d * o.a + self.e * o.d + self.f * o.g,
            e: self.d * o.b + self.e * o.e + self.f * o.h,
            f: self.d * o.c + self.e * o.f + self.f * o.i,
            g: self.g * o.a + self.h * o.d + self.i * o.g,
            h: self.g * o.b + self.h * o.e + self.i * o.h,
            i: self.g * o.c + self.h * o.f + self.i * o.i,
        }
    }
    pub fn apply(&self, v: V3) -> V3 {
        V3::new(
            self.a * v.x + self.b * v.y + self.c * v.z,
            self.d * v.x + self.e * v.y + self.f * v.z,
            self.g * v.x + self.h * v.y + self.i * v.z,
        )
    }
    pub fn det(&self) -> double {
        self.a * (self.e * self.i - self.f * self.h) - self.b * (self.d * self.i - self.f * self.g)
            + self.c * (self.d * self.h - self.e * self.g)
    }
}

/// A placement: `m * p + t`.
#[derive(Clone, Copy, Debug)]
pub struct Xform {
    pub m: M3,
    pub t: V3,
}

impl Xform {
    pub fn identity() -> Xform {
        Xform { m: M3::identity(), t: V3::zero() }
    }
    pub fn new(m: M3, t: V3) -> Xform {
        Xform { m: m, t: t }
    }
    pub fn apply(&self, p: V3) -> V3 {
        self.m.apply(p).add(self.t)
    }
    /// `self` after `inner`: p -> self(inner(p)).
    pub fn then(&self, inner: Xform) -> Xform {
        Xform { m: self.m.mul(inner.m), t: self.m.apply(inner.t).add(self.t) }
    }
}

pub fn clamp(v: double, lo: double, hi: double) -> double {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

pub fn mix(a: double, b: double, t: double) -> double {
    a + (b - a) * t
}

/// A signed power: the superellipse's |c|^p with c's sign.
pub fn spow(c: double, p: double) -> double {
    if c < 0.0 {
        -((-c).powf(p))
    } else {
        c.powf(p)
    }
}

/// Catmull-Rom between b and c (a and d the neighbours).
pub fn catmull(a: double, b: double, c: double, d: double, t: double) -> double {
    let t2 = t * t;
    let t3 = t2 * t;
    0.5 * ((2.0 * b) + (c - a) * t + (2.0 * a - 5.0 * b + 4.0 * c - d) * t2 + (3.0 * b - a - 3.0 * c + d) * t3)
}

pub fn catmull3(a: V3, b: V3, c: V3, d: V3, t: double) -> V3 {
    V3::new(catmull(a.x, b.x, c.x, d.x, t), catmull(a.y, b.y, c.y, d.y, t), catmull(a.z, b.z, c.z, d.z, t))
}
