// SPDX-License-Identifier: MIT
//! Seeded randomness that answers the same on every target: Park and
//! Miller's generator (products below 2^47, exact in an i64 and in a
//! JavaScript number), and smooth value noise built on the same hash.

use ranger::prelude::*;

#[derive(Clone)]
pub struct Rng {
    state: int,
}

impl Rng {
    pub fn new(seed: int) -> Rng {
        let mut s = seed % 2147483647;
        if s < 0 {
            s = s + 2147483647;
        }
        if s == 0 {
            s = 1;
        }
        let mut r = Rng { state: s };
        r.next_int();
        r.next_int();
        r
    }

    pub fn next_int(&mut self) -> int {
        self.state = (self.state * 48271) % 2147483647;
        self.state
    }

    /// A number in (0, 1).
    pub fn next(&mut self) -> double {
        (self.next_int() as double) / 2147483647.0
    }

    /// A number in [lo, hi).
    pub fn range(&mut self, lo: double, hi: double) -> double {
        lo + (hi - lo) * self.next()
    }
}

/// A hash of a lattice point into (0, 1).
pub fn hash3(x: int, y: int, z: int, seed: int) -> double {
    let mut h = (x * 1619 + y * 31337 + z * 6971 + seed * 1013) % 2147483647;
    if h < 0 {
        h = h + 2147483647;
    }
    if h == 0 {
        h = 1;
    }
    h = (h * 48271) % 2147483647;
    h = (h * 48271) % 2147483647;
    h = (h * 48271) % 2147483647;
    (h as double) / 2147483647.0
}

fn smooth(t: double) -> double {
    t * t * (3.0 - 2.0 * t)
}

/// Smooth value noise in [-1, 1].
pub fn noise3(x: double, y: double, z: double, seed: int) -> double {
    let fx = x.floor();
    let fy = y.floor();
    let fz = z.floor();
    let ix = fx as int;
    let iy = fy as int;
    let iz = fz as int;
    let tx = smooth(x - fx);
    let ty = smooth(y - fy);
    let tz = smooth(z - fz);
    let c000 = hash3(ix, iy, iz, seed);
    let c100 = hash3(ix + 1, iy, iz, seed);
    let c010 = hash3(ix, iy + 1, iz, seed);
    let c110 = hash3(ix + 1, iy + 1, iz, seed);
    let c001 = hash3(ix, iy, iz + 1, seed);
    let c101 = hash3(ix + 1, iy, iz + 1, seed);
    let c011 = hash3(ix, iy + 1, iz + 1, seed);
    let c111 = hash3(ix + 1, iy + 1, iz + 1, seed);
    let x00 = c000 + (c100 - c000) * tx;
    let x10 = c010 + (c110 - c010) * tx;
    let x01 = c001 + (c101 - c001) * tx;
    let x11 = c011 + (c111 - c011) * tx;
    let y0 = x00 + (x10 - x00) * ty;
    let y1 = x01 + (x11 - x01) * ty;
    (y0 + (y1 - y0) * tz) * 2.0 - 1.0
}

/// Two octaves of noise.
pub fn fbm3(x: double, y: double, z: double, seed: int) -> double {
    noise3(x, y, z, seed) * 0.7 + noise3(x * 2.03 + 11.0, y * 2.03 + 7.0, z * 2.03 + 3.0, seed + 17) * 0.3
}
