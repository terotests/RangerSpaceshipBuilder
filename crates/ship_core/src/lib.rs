// SPDX-License-Identifier: MIT
//! ship_core: spaceships from compact parametric recipes.
//!
//! A strict Rust module in Ranger's subset (Ranger's
//! docs/plans/PLAN_RUST_SYNTAX.md): cargo builds it (natively, and as
//! WebAssembly for the editor), and `rgrc` writes the same source for
//! Ranger's other targets (JavaScript, C++, Go, …), so a game or a phone
//! app can build ships itself.
//!
//! - `recipe`: the `.ship.json` format and `build(text) -> Ship`
//! - `parts`: the part types (hull, wing, engine, ring, spike, blob, …)
//! - `shapes`: lofts, boxes and beams the parts are made of
//! - `mesh`: the flat-shaded triangle soup with colours and part ids
//! - `material`, `texture`: surface materials and their procedural textures
//! - `gltf`, `png`: the ship as a glTF binary (.glb) with PNG textures
//! - `json`, `math`, `noise`: the small things under them

#[allow(unused_imports)]
use ranger::prelude::*;

pub mod gltf;
pub mod json;
pub mod material;
pub mod math;
pub mod mesh;
pub mod noise;
pub mod parts;
pub mod png;
pub mod recipe;
pub mod shapes;
pub mod texture;
