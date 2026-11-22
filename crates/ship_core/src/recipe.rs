// SPDX-License-Identifier: MIT
//! A ship recipe (`.ship.json`) and how it becomes a mesh.
//!
//! ```json
//! {
//!   "format": "ranger-ship", "version": 1,
//!   "name": "Kipinä", "class": "fighter", "length": 14, "seed": 7,
//!   "palette": { "hull": "#c9ced6", "accent": "#e63946", "dark": "#30343b",
//!                "glow": "#5ad1ff", "glass": "#7fd8ff" },
//!   "parts": [
//!     { "type": "hull", "sections": [[-2, 0.8, 0.5], [0, 1.2, 0.7], [2.5, 0, 0]], "smooth": 2 },
//!     { "type": "wing", "at": [0.4, 0, -0.6], "mirror": true, "span": 2, "sweep": 1 },
//!     { "type": "engine", "at": [0, 0, -2.3], "repeat": { "count": 2, "step": [0.6, 0, 0] } }
//!   ]
//! }
//! ```
//!
//! Every part takes, besides its own parameters (parts.rs):
//!
//! - `at` `[x, y, z]`, `rot` `[x, y, z]` in degrees (x first, then y, then
//!   z), `scale` (a number or `[x, y, z]`)
//! - `mirror`: true adds a copy mirrored across x = 0
//! - `repeat`: `{ count, step: [x, y, z], rotate: [x, y, z], scale }`:
//!   copy k is turned by k × rotate round the ship's origin, moved by
//!   k × step and scaled by scale^k (radial arrays round z for stations)
//! - `color`, `accent`: palette names or "#rrggbb"; `glow`: true draws the
//!   part unlit (lights, windows)
//! - `hidden`: true leaves it out; `name`: a label for the editor
//!
//! The ship's nose points to +z, up is +y; units are free, `length` says
//! how long the ship is in metres (the game scales it).

use crate::json::{Json, JsonReader};
use crate::material::{default_for, resolve, Material};
use crate::math::{M3, V3, Xform};
use crate::mesh::{parse_hex, Mesh};
use crate::parts::{build_part, Params, Swatch};
use ranger::prelude::*;

pub const DEFAULT_PALETTE: &str = "{\"hull\":\"#b8bec7\",\"accent\":\"#d94848\",\"dark\":\"#3a3f47\",\"glow\":\"#62d6ff\",\"glass\":\"#86e0ff\",\"solar\":\"#2b4f9e\"}";

/// A built ship: its mesh, and what went wrong (an empty `error` is a
/// success; `warnings` are parts that were skipped).
pub struct Ship {
    pub mesh: Mesh,
    pub name: string,
    pub class: string,
    pub length: double,
    pub error: string,
    pub warnings: Vec<string>,
    pub part_count: int,
    /// the materials the parts use, in the order of the triangles' `mat`
    pub materials: Vec<Material>,
    /// the texture size in pixels (a power of two)
    pub texture_size: int,
}

impl Ship {
    /// A short JSON summary: name, class, length, triangles, bounds, warnings.
    pub fn info(&self) -> string {
        let b = self.mesh.bounds();
        let mut mats = String::new();
        for (i, m) in self.materials.iter().enumerate() {
            if i > 0 {
                mats.push(',');
            }
            mats.push_str(&format!(
                "{{\"name\":{},\"pattern\":{},\"metal\":{},\"rough\":{}}}",
                crate::json::quote(&m.name),
                crate::json::quote(&m.pattern),
                crate::json::num(m.metal),
                crate::json::num(m.rough)
            ));
        }
        let mut w = String::new();
        for (i, s) in self.warnings.iter().enumerate() {
            if i > 0 {
                w.push(',');
            }
            w.push_str(&crate::json::quote(s));
        }
        format!(
            "{{\"name\":{},\"class\":{},\"length\":{},\"triangles\":{},\"parts\":{},\"min\":[{},{},{}],\"max\":[{},{},{}],\"error\":{},\"warnings\":[{}],\"materials\":[{}],\"textureSize\":{}}}",
            crate::json::quote(&self.name),
            crate::json::quote(&self.class),
            crate::json::num(self.length),
            self.mesh.count(),
            self.part_count,
            crate::json::num(b[0].x),
            crate::json::num(b[0].y),
            crate::json::num(b[0].z),
            crate::json::num(b[1].x),
            crate::json::num(b[1].y),
            crate::json::num(b[1].z),
            crate::json::quote(&self.error),
            w,
            mats,
            self.texture_size
        )
    }
}

fn palette_of(recipe: &Json) -> Vec<Swatch> {
    let mut out: Vec<Swatch> = Vec::new();
    let mut reader = JsonReader::new(DEFAULT_PALETTE);
    let defaults = reader.parse();
    let given = recipe.get("palette");
    for layer in vec![defaults, given].iter() {
        if let Json::Obj(members) = layer {
            for m in members.iter() {
                if let Some(c) = parse_hex(&m.value.str_or("")) {
                    let mut found = false;
                    for s in out.iter_mut() {
                        if s.name == m.key {
                            s.col = c;
                            found = true;
                        }
                    }
                    if !found {
                        out.push(Swatch { name: m.key.clone(), col: c });
                    }
                }
            }
        }
    }
    out
}

fn v3_of(j: &Json, fallback: V3) -> V3 {
    match j {
        Json::Num(n) => V3::new(*n, *n, *n),
        Json::Arr(_) => V3::new(j.at_num(0, fallback.x), j.at_num(1, fallback.y), j.at_num(2, fallback.z)),
        _ => fallback,
    }
}

/// Builds the ship described by the recipe text.
pub fn build(text: &str) -> Ship {
    let mut reader = JsonReader::new(text);
    let recipe = reader.parse();
    let mut ship = Ship {
        mesh: Mesh::new(),
        name: recipe.get("name").str_or("Alus"),
        class: recipe.get("class").str_or("fighter"),
        length: recipe.get("length").num_or(20.0),
        error: reader.error.clone(),
        warnings: Vec::new(),
        part_count: 0,
        materials: Vec::new(),
        texture_size: 256,
    };
    if !ship.error.is_empty() {
        return ship;
    }
    let palette = palette_of(&recipe);
    let seed = recipe.get("seed").num_or(1.0) as int;
    let parts = recipe.get("parts").items();
    ship.part_count = parts.len() as int;
    for (index, part) in parts.iter().enumerate() {
        if part.get("hidden").bool_or(false) {
            continue;
        }
        let kind = part.get("type").str_or("");
        let params = Params {
            json: part.clone(),
            palette: palette.clone(),
            seed: seed * 7919 + (index as int) * 104729 + (part.get("seed").num_or(0.0) as int) * 31,
        };
        let mut local = Mesh::new();
        if !build_part(&kind, &params, &mut local) {
            ship.warnings.push(format!("part {}: unknown type \"{}\"", index, kind));
            continue;
        }
        // textures are projected in the part's own frame
        local.lpos = local.pos.clone();
        let mat_name = part.get("material").str_or(&recipe.get("material").str_or(&default_for(&kind)));
        let mut mat: int = -1;
        for (mi, m) in ship.materials.iter().enumerate() {
            if m.name == mat_name {
                mat = mi as int;
            }
        }
        if mat < 0 {
            ship.materials.push(resolve(&recipe, &mat_name));
            mat = (ship.materials.len() as int) - 1;
        }
        // the part's own placement: scale, turn, move
        let scale = v3_of(&part.get("scale"), V3::new(1.0, 1.0, 1.0));
        let rot = v3_of(&part.get("rot"), V3::zero());
        let at = v3_of(&part.get("at"), V3::zero());
        let place = Xform::new(M3::euler(rot).mul(M3::scale(scale)), at);
        // repeats round the ship's origin
        let rep = part.get("repeat");
        let mut count = rep.get("count").num_or(1.0) as int;
        if count < 1 {
            count = 1;
        }
        if count > 64 {
            count = 64;
        }
        let step = v3_of(&rep.get("step"), V3::zero());
        let turn = v3_of(&rep.get("rotate"), V3::zero());
        let grow = rep.get("scale").num_or(1.0);
        let mirror = part.get("mirror").bool_or(false);
        let mut k: int = 0;
        while k < count {
            let kf = k as double;
            let g = grow.powf(kf);
            let copy = Xform::new(M3::euler(turn.mul(kf)).mul(M3::scale(V3::new(g, g, g))), step.mul(kf));
            let x = copy.then(place);
            ship.mesh.append(&local, x, index as int, mat);
            if mirror {
                let m = Xform::new(M3::scale(V3::new(-1.0, 1.0, 1.0)), V3::zero()).then(x);
                ship.mesh.append(&local, m, index as int, mat);
            }
            k += 1;
        }
    }
    let ts = recipe.get("textureSize").num_or(256.0) as int;
    ship.texture_size = if ts <= 64 { 64 } else if ts <= 128 { 128 } else if ts <= 256 { 256 } else if ts <= 512 { 512 } else { 1024 };
    project_uvs(&mut ship);
    ship
}

/// Texture coordinates: each triangle projected along the axis its face
/// mostly points to, in its part's frame. One tile is about 2.5 m on a small
/// ship and a fourteenth of a big ship's length, times the material's scale.
fn project_uvs(ship: &mut Ship) {
    let b = ship.mesh.bounds();
    let model_len = (b[1].z - b[0].z).max(0.001);
    let metres = if ship.length > 0.0 { ship.length } else { 20.0 };
    let tile_m = (metres / 14.0).max(2.5);
    let tile = tile_m * model_len / metres;
    let n = ship.mesh.count();
    let mut t: int = 0;
    while t < n {
        let a = ship.mesh.local(t, 0);
        let bb = ship.mesh.local(t, 1);
        let c = ship.mesh.local(t, 2);
        let nr = bb.sub(a).cross(c.sub(a)).norm();
        // rounded, so faces at exactly 45° pick the same axis on every
        // target (sin and cos may differ in the last bit)
        let ax = (nr.x.abs() * 10000.0).round();
        let ay = (nr.y.abs() * 10000.0).round();
        let az = (nr.z.abs() * 10000.0).round();
        let mi = ship.mesh.mat[t as usize];
        let scale = if mi >= 0 && (mi as usize) < ship.materials.len() { ship.materials[mi as usize].scale } else { 1.0 };
        let k = 1.0 / (tile * scale);
        let corners = vec![a, bb, c];
        let mut j: int = 0;
        while j < 3 {
            let q = corners[j as usize];
            let mut u = q.x;
            let mut v = q.y;
            if ax >= ay && ax >= az {
                u = q.z;
                v = q.y;
            } else if ay >= az {
                u = q.x;
                v = q.z;
            }
            let i = (t * 6 + j * 2) as usize;
            ship.mesh.uv[i] = u * k;
            ship.mesh.uv[i + 1] = -v * k;
            j += 1;
        }
        t += 1;
    }
}
