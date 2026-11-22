// SPDX-License-Identifier: MIT
//! A flat-shaded triangle soup: every triangle has its own colour (linear
//! RGB), a glow flag (drawn unlit, the engines' flames and the windows) and
//! the index of the recipe part it came from (the editor picks parts by it).

use crate::math::{Xform, V3};
use ranger::prelude::*;

/// A colour in linear RGB.
#[derive(Clone, Copy, Debug)]
pub struct Rgb {
    pub r: double,
    pub g: double,
    pub b: double,
}

impl Rgb {
    pub fn new(r: double, g: double, b: double) -> Rgb {
        Rgb { r: r, g: g, b: b }
    }
    pub fn scale(&self, k: double) -> Rgb {
        Rgb::new(clamp01(self.r * k), clamp01(self.g * k), clamp01(self.b * k))
    }
    pub fn mix(&self, o: Rgb, t: double) -> Rgb {
        Rgb::new(self.r + (o.r - self.r) * t, self.g + (o.g - self.g) * t, self.b + (o.b - self.b) * t)
    }
}

fn clamp01(v: double) -> double {
    if v < 0.0 {
        0.0
    } else if v > 1.0 {
        1.0
    } else {
        v
    }
}

fn hex_digit(c: char) -> int {
    let code = c as int;
    if code >= 48 && code <= 57 {
        return code - 48;
    }
    if code >= 97 && code <= 102 {
        return code - 87;
    }
    if code >= 65 && code <= 70 {
        return code - 55;
    }
    -1
}

fn to_linear(c: double) -> double {
    if c <= 0.04045 {
        c / 12.92
    } else {
        ((c + 0.055) / 1.055).powf(2.4)
    }
}

/// "#rrggbb" or "#rgb" (sRGB, as CSS writes it) as a linear colour; `None`
/// for anything else.
pub fn parse_hex(s: &str) -> Option<Rgb> {
    let mut digits: Vec<int> = Vec::new();
    let mut first = true;
    for c in s.chars() {
        if first {
            first = false;
            if c == '#' {
                continue;
            }
        }
        let d = hex_digit(c);
        if d < 0 {
            return None;
        }
        digits.push(d);
    }
    let n = digits.len();
    if n == 6 {
        let r = (digits[0] * 16 + digits[1]) as double / 255.0;
        let g = (digits[2] * 16 + digits[3]) as double / 255.0;
        let b = (digits[4] * 16 + digits[5]) as double / 255.0;
        return Some(Rgb::new(to_linear(r), to_linear(g), to_linear(b)));
    }
    if n == 3 {
        let r = (digits[0] * 17) as double / 255.0;
        let g = (digits[1] * 17) as double / 255.0;
        let b = (digits[2] * 17) as double / 255.0;
        return Some(Rgb::new(to_linear(r), to_linear(g), to_linear(b)));
    }
    None
}

pub struct Mesh {
    /// 9 numbers a triangle: three corners
    pub pos: Vec<double>,
    /// 3 numbers a triangle: its linear colour
    pub col: Vec<double>,
    pub glow: Vec<bool>,
    pub part: Vec<int>,
}

impl Mesh {
    pub fn new() -> Mesh {
        Mesh { pos: Vec::new(), col: Vec::new(), glow: Vec::new(), part: Vec::new() }
    }

    pub fn count(&self) -> int {
        self.glow.len() as int
    }

    pub fn corner(&self, t: int, k: int) -> V3 {
        let i = (t * 9 + k * 3) as usize;
        V3::new(self.pos[i], self.pos[i + 1], self.pos[i + 2])
    }

    pub fn color(&self, t: int) -> Rgb {
        let i = (t * 3) as usize;
        Rgb::new(self.col[i], self.col[i + 1], self.col[i + 2])
    }

    /// Adds a triangle (counter-clockwise seen from outside). Triangles with
    /// no area are left out.
    pub fn tri(&mut self, a: V3, b: V3, c: V3, col: Rgb, glow: bool) {
        let n = b.sub(a).cross(c.sub(a));
        if n.len() < 1.0e-12 {
            return;
        }
        self.pos.push(a.x);
        self.pos.push(a.y);
        self.pos.push(a.z);
        self.pos.push(b.x);
        self.pos.push(b.y);
        self.pos.push(b.z);
        self.pos.push(c.x);
        self.pos.push(c.y);
        self.pos.push(c.z);
        self.col.push(col.r);
        self.col.push(col.g);
        self.col.push(col.b);
        self.glow.push(glow);
        self.part.push(0);
    }

    pub fn quad(&mut self, a: V3, b: V3, c: V3, d: V3, col: Rgb, glow: bool) {
        self.tri(a, b, c, col, glow);
        self.tri(a, c, d, col, glow);
    }

    /// Copies `src` in, placed by `x`, every triangle marked as part `part`.
    /// A mirroring placement turns the triangles round so they still face out.
    pub fn append(&mut self, src: &Mesh, x: Xform, part: int) {
        let flip = x.m.det() < 0.0;
        let n = src.count();
        let mut t: int = 0;
        while t < n {
            let a = x.apply(src.corner(t, 0));
            let b = x.apply(src.corner(t, 1));
            let c = x.apply(src.corner(t, 2));
            let col = src.color(t);
            let g = src.glow[t as usize];
            let before = self.count();
            if flip {
                self.tri(a, c, b, col, g);
            } else {
                self.tri(a, b, c, col, g);
            }
            if self.count() > before {
                let last = self.part.len() - 1;
                self.part[last] = part;
            }
            t += 1;
        }
    }

    /// The face normal of triangle `t`.
    pub fn normal(&self, t: int) -> V3 {
        let a = self.corner(t, 0);
        let b = self.corner(t, 1);
        let c = self.corner(t, 2);
        b.sub(a).cross(c.sub(a)).norm()
    }

    /// The smallest box round every corner: [min, max].
    pub fn bounds(&self) -> Vec<V3> {
        let mut lo = V3::new(0.0, 0.0, 0.0);
        let mut hi = V3::new(0.0, 0.0, 0.0);
        let n = self.count();
        let mut t: int = 0;
        while t < n {
            let mut k: int = 0;
            while k < 3 {
                let p = self.corner(t, k);
                if t == 0 && k == 0 {
                    lo = p;
                    hi = p;
                }
                if p.x < lo.x {
                    lo.x = p.x;
                }
                if p.y < lo.y {
                    lo.y = p.y;
                }
                if p.z < lo.z {
                    lo.z = p.z;
                }
                if p.x > hi.x {
                    hi.x = p.x;
                }
                if p.y > hi.y {
                    hi.y = p.y;
                }
                if p.z > hi.z {
                    hi.z = p.z;
                }
                k += 1;
            }
            t += 1;
        }
        vec![lo, hi]
    }
}
