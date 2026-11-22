// SPDX-License-Identifier: MIT
//! Surface materials: a texture pattern (texture.rs) and how the surface
//! takes light (metal, roughness). A part names its material with
//! `"material"`; a recipe can define its own under `"materials"`:
//!
//! ```json
//! "materials": {
//!   "kuori": { "pattern": "plates", "scale": 1.5, "metal": 0.7, "rough": 0.35, "grime": 0.4, "depth": 1 }
//! }
//! ```
//!
//! The built-in materials are named after their patterns: `plates`,
//! `armor`, `hex`, `organic`, `carbon`, `solar`, `vents`, `glass`, `paint`,
//! `hazard`. The texture holds brightness detail only: the part's colour
//! (the palette) is multiplied with it, so one material serves every colour.

use crate::json::Json;
use ranger::prelude::*;

#[derive(Clone, Debug)]
pub struct Material {
    pub name: string,
    pub pattern: string,
    /// the size of one texture tile, relative to the ship's own default
    pub scale: double,
    pub metal: double,
    pub rough: double,
    /// dirt, streaks and wear, 0..1
    pub grime: double,
    /// how deep the grooves look in the normal map, 0..3
    pub depth: double,
    pub seed: int,
}

pub const PATTERNS: &str = "plates armor hex organic carbon solar vents glass paint hazard";

/// The built-in material `pattern`.
pub fn builtin(pattern: &str) -> Material {
    let mut m = Material {
        name: pattern.to_string(),
        pattern: pattern.to_string(),
        scale: 1.0,
        metal: 0.5,
        rough: 0.45,
        grime: 0.25,
        depth: 1.0,
        seed: 1,
    };
    if pattern == "armor" {
        m.metal = 0.75;
        m.rough = 0.35;
        m.scale = 1.6;
        m.grime = 0.35;
    } else if pattern == "hex" {
        m.metal = 0.2;
        m.rough = 0.4;
        m.scale = 0.7;
        m.grime = 0.1;
    } else if pattern == "organic" {
        m.metal = 0.0;
        m.rough = 0.35;
        m.scale = 1.2;
        m.grime = 0.2;
        m.depth = 1.4;
    } else if pattern == "carbon" {
        m.metal = 0.1;
        m.rough = 0.3;
        m.scale = 0.35;
        m.grime = 0.05;
        m.depth = 0.5;
    } else if pattern == "solar" {
        m.metal = 0.4;
        m.rough = 0.15;
        m.scale = 0.6;
        m.grime = 0.0;
        m.depth = 0.4;
    } else if pattern == "vents" {
        m.metal = 0.6;
        m.rough = 0.5;
        m.scale = 0.5;
        m.grime = 0.3;
        m.depth = 1.6;
    } else if pattern == "glass" {
        m.metal = 0.1;
        m.rough = 0.05;
        m.grime = 0.0;
        m.depth = 0.2;
    } else if pattern == "paint" {
        m.metal = 0.15;
        m.rough = 0.55;
        m.grime = 0.15;
        m.depth = 0.3;
    } else if pattern == "hazard" {
        m.metal = 0.2;
        m.rough = 0.5;
        m.scale = 0.6;
        m.grime = 0.3;
        m.depth = 0.3;
    }
    m
}

pub fn is_pattern(p: &str) -> bool {
    for w in PATTERNS.split(' ') {
        if w == p {
            return true;
        }
    }
    false
}

/// The material a part type gets when it names none.
pub fn default_for(kind: &str) -> string {
    if kind == "cockpit" {
        return "glass".to_string();
    }
    if kind == "panel" {
        return "solar".to_string();
    }
    if kind == "engine" {
        return "vents".to_string();
    }
    if kind == "turret" {
        return "armor".to_string();
    }
    if kind == "blob" || kind == "spike" || kind == "truss" || kind == "dish" || kind == "cylinder" {
        return "paint".to_string();
    }
    "plates".to_string()
}

/// The recipe's own material `name` (from `"materials"`), or the built-in
/// one; an unknown name falls back to `plates`.
pub fn resolve(recipe: &Json, name: &str) -> Material {
    let def = recipe.get("materials").get(name);
    if def.is_null() {
        if is_pattern(name) {
            return builtin(name);
        }
        let mut m = builtin("plates");
        m.name = name.to_string();
        return m;
    }
    let pattern = def.get("pattern").str_or("plates");
    let base = if is_pattern(&pattern) { builtin(&pattern) } else { builtin("plates") };
    Material {
        name: name.to_string(),
        pattern: base.pattern.clone(),
        scale: def.get("scale").num_or(base.scale).max(0.05),
        metal: def.get("metal").num_or(base.metal),
        rough: def.get("rough").num_or(base.rough),
        grime: def.get("grime").num_or(base.grime),
        depth: def.get("depth").num_or(base.depth),
        seed: def.get("seed").num_or(1.0) as int,
    }
}
