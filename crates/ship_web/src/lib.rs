// SPDX-License-Identifier: MIT
//! ship_core as a WebAssembly module (wasm32-unknown-unknown) behind a C
//! ABI, for the editor (web/editor.js):
//!
//!   ship_alloc(n) -> ptr / ship_free(ptr, n)  a buffer the page writes the recipe into
//!   ship_build(ptr, len)                      builds the recipe; 0 ok, 1 error (see the info)
//!   ship_info_ptr / ship_info_len             the build's summary as JSON (recipe::Ship::info)
//!   ship_mesh_ptr / ship_mesh_len             the triangles for WebGL, f32s, 14 a corner:
//!                                             x y z, nx ny nz, r g b (linear), u v, part, glow, material;
//!                                             grouped by material (the info's "materials" order)
//!   ship_ranges_ptr / ship_ranges_len         u32 pairs: first corner and corner count of each material
//!   ship_tex(material) -> size, ship_tex_ptr  that material's maps, RGB, size × size each:
//!                                             base colour, then metal/roughness, then normals
//!   ship_glb(textures) -> len, ship_glb_ptr   the ship as a .glb file (with PNG textures when 1)

use std::cell::RefCell;

struct State {
    info: Vec<u8>,
    mesh: Vec<f32>,
    ranges: Vec<u32>,
    tex: Vec<u8>,
    glb: Vec<u8>,
    recipe: String,
}

thread_local! {
    static STATE: RefCell<State> = RefCell::new(State { info: Vec::new(), mesh: Vec::new(), ranges: Vec::new(), tex: Vec::new(), glb: Vec::new(), recipe: String::new() });
}

#[no_mangle]
pub extern "C" fn ship_alloc(n: usize) -> *mut u8 {
    let mut v: Vec<u8> = Vec::with_capacity(n.max(1));
    let p = v.as_mut_ptr();
    std::mem::forget(v);
    p
}

/// # Safety
/// `p` must come from `ship_alloc(n)`.
#[no_mangle]
pub unsafe extern "C" fn ship_free(p: *mut u8, n: usize) {
    drop(Vec::from_raw_parts(p, 0, n.max(1)));
}

/// The render buffer for a built ship: 14 floats a corner, grouped by
/// material, and each material's [first corner, corner count].
pub fn render_buffer(ship: &ship_core::recipe::Ship) -> (Vec<f32>, Vec<u32>) {
    let m = &ship.mesh;
    let n = m.count();
    let mut out = Vec::with_capacity((n * 42) as usize);
    let mut ranges = Vec::new();
    for mat in 0..(ship.materials.len() as i64).max(1) {
        let first = out.len() / 14;
        for t in 0..n {
            if m.mat[t as usize] != mat {
                continue;
            }
            let nr = m.normal(t);
            let c = m.color(t);
            let part = m.part[t as usize] as f32;
            let glow = if m.glow[t as usize] { 1.0 } else { 0.0 };
            for k in 0..3 {
                let p = m.corner(t, k);
                let ui = (t * 6 + k * 2) as usize;
                out.extend_from_slice(&[p.x as f32, p.y as f32, p.z as f32, nr.x as f32, nr.y as f32, nr.z as f32, c.r as f32, c.g as f32, c.b as f32, m.uv[ui] as f32, m.uv[ui + 1] as f32, part, glow, mat as f32]);
            }
        }
        ranges.push(first as u32);
        ranges.push((out.len() / 14 - first) as u32);
    }
    (out, ranges)
}

/// # Safety
/// `p` must point to `len` readable bytes of UTF-8.
#[no_mangle]
pub unsafe extern "C" fn ship_build(p: *const u8, len: usize) -> i32 {
    let text = String::from_utf8_lossy(std::slice::from_raw_parts(p, len)).to_string();
    let ship = ship_core::recipe::build(&text);
    let ok = ship.error.is_empty();
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        s.info = ship.info().into_bytes();
        let (mesh, ranges) = render_buffer(&ship);
        s.mesh = mesh;
        s.ranges = ranges;
        s.glb = Vec::new();
        s.recipe = text;
    });
    if ok { 0 } else { 1 }
}

#[no_mangle]
pub extern "C" fn ship_info_ptr() -> *const u8 {
    STATE.with(|s| s.borrow().info.as_ptr())
}

#[no_mangle]
pub extern "C" fn ship_info_len() -> usize {
    STATE.with(|s| s.borrow().info.len())
}

#[no_mangle]
pub extern "C" fn ship_mesh_ptr() -> *const f32 {
    STATE.with(|s| s.borrow().mesh.as_ptr())
}

#[no_mangle]
pub extern "C" fn ship_mesh_len() -> usize {
    STATE.with(|s| s.borrow().mesh.len())
}

#[no_mangle]
pub extern "C" fn ship_ranges_ptr() -> *const u32 {
    STATE.with(|s| s.borrow().ranges.as_ptr())
}

#[no_mangle]
pub extern "C" fn ship_ranges_len() -> usize {
    STATE.with(|s| s.borrow().ranges.len())
}

/// Makes the maps of material `mat` of the last built recipe; their size.
#[no_mangle]
pub extern "C" fn ship_tex(mat: usize) -> usize {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let ship = ship_core::recipe::build(&s.recipe);
        if mat >= ship.materials.len() {
            s.tex = Vec::new();
            return 0;
        }
        let set = ship_core::texture::generate(&ship.materials[mat], ship.texture_size);
        let mut t = set.base;
        t.extend_from_slice(&set.orm);
        t.extend_from_slice(&set.normal);
        s.tex = t;
        set.size as usize
    })
}

#[no_mangle]
pub extern "C" fn ship_tex_ptr() -> *const u8 {
    STATE.with(|s| s.borrow().tex.as_ptr())
}

/// Writes the .glb of the last built recipe (with textures when 1); its length.
#[no_mangle]
pub extern "C" fn ship_glb(textures: i32) -> usize {
    STATE.with(|s| {
        let mut s = s.borrow_mut();
        let ship = ship_core::recipe::build(&s.recipe);
        s.glb = ship_core::gltf::write_glb_with(&ship, textures != 0);
        s.glb.len()
    })
}

#[no_mangle]
pub extern "C" fn ship_glb_ptr() -> *const u8 {
    STATE.with(|s| s.borrow().glb.as_ptr())
}
