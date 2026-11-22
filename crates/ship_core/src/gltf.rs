// SPDX-License-Identifier: MIT
//! A glTF 2.0 binary (.glb) of a ship mesh: one node, one mesh with a
//! primitive per material, flat-shaded triangles with vertex colours
//! (COLOR_0) and texture coordinates:
//!
//! - one lit material per ship material: metal and roughness factors, or
//!   (with textures) its procedural base colour, metal/roughness and normal
//!   maps as PNG images in the file
//! - last `glow`: `KHR_materials_unlit` with an emissive factor, for the
//!   engines' exhaust, windows and lights
//!
//! RangerGames' engine reads exactly this (crates/rg_engine/src/gltf.rs:
//! POSITION, NORMAL, COLOR_0, baseColorFactor, emissiveFactor,
//! KHR_materials_unlit), and so do three.js, Blender and the rest.
//!
//! The floats are written by arithmetic (no `to_bits`), so every Ranger
//! target writes the same bytes.

use crate::json::{num, quote};
use crate::mesh::Mesh;
use crate::recipe::Ship;
use ranger::prelude::*;

/// The IEEE 754 single precision bits of `x` (round to nearest; tiny
/// numbers become 0).
pub fn f32_bits(x: double) -> int {
    if x == 0.0 || x != x {
        return 0;
    }
    let mut sign: int = 0;
    let mut a = x;
    if a < 0.0 {
        sign = 1;
        a = -a;
    }
    let mut e: int = 0;
    while a >= 2.0 && e < 200 {
        a = a / 2.0;
        e += 1;
    }
    while a < 1.0 && e > -200 {
        a = a * 2.0;
        e -= 1;
    }
    let mut biased = e + 127;
    if biased <= 0 {
        return sign * 2147483648;
    }
    let mut mant = ((a - 1.0) * 8388608.0).round() as int;
    if mant >= 8388608 {
        mant = 0;
        biased += 1;
    }
    if biased >= 255 {
        return sign * 2147483648 + 255 * 8388608;
    }
    sign * 2147483648 + biased * 8388608 + mant
}

/// `x` rounded to the nearest single precision number (what the file holds).
pub fn f32_round(x: double) -> double {
    let bits = f32_bits(x);
    if bits == 0 {
        return 0.0;
    }
    let sign = if bits >= 2147483648 { -1.0 } else { 1.0 };
    let rest = bits % 2147483648;
    let e = rest / 8388608;
    let mant = rest % 8388608;
    let mut v = 1.0 + (mant as double) / 8388608.0;
    let mut k = e - 127;
    while k > 0 {
        v = v * 2.0;
        k -= 1;
    }
    while k < 0 {
        v = v / 2.0;
        k += 1;
    }
    sign * v
}

fn put_u32(buf: &mut Vec<u8>, at: int, v: int) {
    let mut x = v;
    let mut i: int = 0;
    while i < 4 {
        buf[(at + i) as usize] = (x % 256) as u8;
        x = x / 256;
        i += 1;
    }
}

fn put_f32(buf: &mut Vec<u8>, at: int, v: double) {
    put_u32(buf, at, f32_bits(v));
}

/// The triangles of one primitive: positions, normals, colours (3 numbers
/// a corner) and texture coordinates (2 a corner).
struct Prim {
    pos: Vec<double>,
    nrm: Vec<double>,
    col: Vec<double>,
    uv: Vec<double>,
    count: int,
    min: Vec<double>,
    max: Vec<double>,
    /// the ship material, or -1 for the glow primitive
    mat: int,
}

/// The triangles with `glow`, and (when not glowing) material `mat`.
fn prim_of(m: &Mesh, glow: bool, mat: int) -> Prim {
    let mut p = Prim { pos: Vec::new(), nrm: Vec::new(), col: Vec::new(), uv: Vec::new(), count: 0, min: vec![0.0, 0.0, 0.0], max: vec![0.0, 0.0, 0.0], mat: if glow { -1 } else { mat } };
    let n = m.count();
    let mut t: int = 0;
    while t < n {
        if m.glow[t as usize] == glow && (glow || m.mat[t as usize] == mat) {
            let nr = m.normal(t);
            let c = m.color(t);
            let mut k: int = 0;
            while k < 3 {
                let q = m.corner(t, k);
                let vals = vec![q.x, q.y, q.z];
                let mut a: int = 0;
                while a < 3 {
                    let v = f32_round(vals[a as usize]);
                    if p.count == 0 && k == 0 {
                        p.min[a as usize] = v;
                        p.max[a as usize] = v;
                    }
                    if v < p.min[a as usize] {
                        p.min[a as usize] = v;
                    }
                    if v > p.max[a as usize] {
                        p.max[a as usize] = v;
                    }
                    a += 1;
                }
                p.pos.push(q.x);
                p.pos.push(q.y);
                p.pos.push(q.z);
                p.nrm.push(nr.x);
                p.nrm.push(nr.y);
                p.nrm.push(nr.z);
                p.col.push(c.r);
                p.col.push(c.g);
                p.col.push(c.b);
                let ui = (t * 6 + k * 2) as usize;
                p.uv.push(m.uv[ui]);
                p.uv.push(m.uv[ui + 1]);
                k += 1;
            }
            p.count += 1;
        }
        t += 1;
    }
    p
}

/// Exact, so the bounds really hold every corner.
fn vec3_json(v: &Vec<double>) -> string {
    format!("[{},{},{}]", v[0], v[1], v[2])
}

fn copy_bytes(dst: &mut Vec<u8>, at: int, src: &Vec<u8>) {
    let n = src.len() as int;
    let mut i: int = 0;
    while i < n {
        dst[(at + i) as usize] = src[i as usize];
        i += 1;
    }
}

/// The ship as a .glb file with vertex colours only (what RangerGames'
/// engine draws: it reads no textures).
pub fn write_glb(ship: &Ship) -> Vec<u8> {
    write_glb_with(ship, false)
}

/// The ship as a .glb file; with `textures` each material carries its
/// procedural base colour, metal/roughness and normal maps (PNG).
pub fn write_glb_with(ship: &Ship, textures: bool) -> Vec<u8> {
    let mut prims: Vec<Prim> = Vec::new();
    let mut mi: int = 0;
    while mi < (ship.materials.len() as int) {
        let p = prim_of(&ship.mesh, false, mi);
        if p.count > 0 {
            prims.push(p);
        }
        mi += 1;
    }
    let g = prim_of(&ship.mesh, true, -1);
    if g.count > 0 {
        prims.push(g);
    }
    // the images: three per textured material, in the order of `prims`
    let mut images: Vec<Vec<u8>> = Vec::new();
    if textures {
        for p in prims.iter() {
            if p.mat >= 0 {
                let set = crate::texture::generate(&ship.materials[p.mat as usize], ship.texture_size);
                images.push(crate::png::write_rgb(set.size, set.size, &set.base));
                images.push(crate::png::write_rgb(set.size, set.size, &set.orm));
                images.push(crate::png::write_rgb(set.size, set.size, &set.normal));
            }
        }
    }
    let mut views = String::new();
    let mut accessors = String::new();
    let mut primitives = String::new();
    let mut materials = String::new();
    let mut offset: int = 0;
    let mut acc: int = 0;
    let mut tex: int = 0;
    for (pi, p) in prims.iter().enumerate() {
        let verts = p.count * 3;
        let mut k: int = 0;
        while k < 4 {
            let comps = if k == 3 { 2 } else { 3 };
            let bytes = verts * comps * 4;
            if acc > 0 {
                views.push(',');
                accessors.push(',');
            }
            views.push_str(&format!("{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{},\"target\":34962}}", offset, bytes));
            if k == 0 {
                accessors.push_str(&format!(
                    "{{\"bufferView\":{},\"componentType\":5126,\"count\":{},\"type\":\"VEC3\",\"min\":{},\"max\":{}}}",
                    acc,
                    verts,
                    vec3_json(&p.min),
                    vec3_json(&p.max)
                ));
            } else {
                accessors.push_str(&format!("{{\"bufferView\":{},\"componentType\":5126,\"count\":{},\"type\":\"{}\"}}", acc, verts, if k == 3 { "VEC2" } else { "VEC3" }));
            }
            offset += bytes;
            acc += 1;
            k += 1;
        }
        if pi > 0 {
            primitives.push(',');
            materials.push(',');
        }
        primitives.push_str(&format!(
            "{{\"attributes\":{{\"POSITION\":{},\"NORMAL\":{},\"COLOR_0\":{},\"TEXCOORD_0\":{}}},\"material\":{},\"mode\":4}}",
            acc - 4,
            acc - 3,
            acc - 2,
            acc - 1,
            pi
        ));
        if p.mat < 0 {
            materials.push_str("{\"name\":\"glow\",\"pbrMetallicRoughness\":{\"baseColorFactor\":[1,1,1,1],\"metallicFactor\":0,\"roughnessFactor\":1},\"emissiveFactor\":[1,1,1],\"extensions\":{\"KHR_materials_unlit\":{}}}");
        } else {
            let m = &ship.materials[p.mat as usize];
            if textures {
                materials.push_str(&format!(
                    "{{\"name\":{},\"pbrMetallicRoughness\":{{\"baseColorFactor\":[1,1,1,1],\"baseColorTexture\":{{\"index\":{}}},\"metallicFactor\":1,\"roughnessFactor\":1,\"metallicRoughnessTexture\":{{\"index\":{}}}}},\"normalTexture\":{{\"index\":{}}}}}",
                    quote(&m.name),
                    tex,
                    tex + 1,
                    tex + 2
                ));
                tex += 3;
            } else {
                materials.push_str(&format!(
                    "{{\"name\":{},\"pbrMetallicRoughness\":{{\"baseColorFactor\":[1,1,1,1],\"metallicFactor\":{},\"roughnessFactor\":{}}}}}",
                    quote(&m.name),
                    num(m.metal),
                    num(m.rough)
                ));
            }
        }
    }
    // images after the vertex data, each padded to 4 bytes
    let mut image_json = String::new();
    let mut texture_json = String::new();
    let mut img_offsets: Vec<int> = Vec::new();
    for (ii, img) in images.iter().enumerate() {
        let len = img.len() as int;
        views.push_str(&format!(",{{\"buffer\":0,\"byteOffset\":{},\"byteLength\":{}}}", offset, len));
        if ii > 0 {
            image_json.push(',');
            texture_json.push(',');
        }
        image_json.push_str(&format!("{{\"bufferView\":{},\"mimeType\":\"image/png\"}}", acc + (ii as int)));
        texture_json.push_str(&format!("{{\"sampler\":0,\"source\":{}}}", ii));
        img_offsets.push(offset);
        offset += len + (4 - len % 4) % 4;
    }
    let tex_json = if images.is_empty() {
        String::new()
    } else {
        format!(",\"samplers\":[{{\"magFilter\":9729,\"minFilter\":9987,\"wrapS\":10497,\"wrapT\":10497}}],\"images\":[{}],\"textures\":[{}]", image_json, texture_json)
    };
    let bin_len = offset;
    let json = format!(
        "{{\"asset\":{{\"version\":\"2.0\",\"generator\":\"RangerSpaceshipBuilder\"}},\"extensionsUsed\":[\"KHR_materials_unlit\"],\"scene\":0,\"scenes\":[{{\"nodes\":[0]}}],\"nodes\":[{{\"name\":{},\"mesh\":0,\"extras\":{{\"class\":{},\"length\":{}}}}}],\"meshes\":[{{\"name\":{},\"primitives\":[{}]}}],\"materials\":[{}],\"buffers\":[{{\"byteLength\":{}}}],\"bufferViews\":[{}],\"accessors\":[{}]{}}}",
        quote(&ship.name),
        quote(&ship.class),
        num(ship.length),
        quote(&ship.name),
        primitives,
        materials,
        bin_len,
        views,
        accessors,
        tex_json
    );
    let jb = json.as_bytes();
    let jlen = jb.len() as int;
    let jpad = (4 - jlen % 4) % 4;
    let jtotal = jlen + jpad;
    let total = 12 + 8 + jtotal + 8 + bin_len;
    let mut buf = vec![0u8; total as usize];
    // header: "glTF", version 2, total length
    put_u32(&mut buf, 0, 1179937895);
    put_u32(&mut buf, 4, 2);
    put_u32(&mut buf, 8, total);
    // the JSON chunk, padded with spaces
    put_u32(&mut buf, 12, jtotal);
    put_u32(&mut buf, 16, 1313821514);
    let mut i: int = 0;
    while i < jtotal {
        buf[(20 + i) as usize] = if i < jlen { jb[i as usize] } else { 32 };
        i += 1;
    }
    // the binary chunk
    let b0 = 20 + jtotal;
    put_u32(&mut buf, b0, bin_len);
    put_u32(&mut buf, b0 + 4, 5130562);
    let mut at = b0 + 8;
    for p in prims.iter() {
        for arr in vec![&p.pos, &p.nrm, &p.col, &p.uv].iter() {
            for v in arr.iter() {
                put_f32(&mut buf, at, *v);
                at += 4;
            }
        }
    }
    for (ii, img) in images.iter().enumerate() {
        copy_bytes(&mut buf, b0 + 8 + img_offsets[ii], img);
    }
    buf
}
