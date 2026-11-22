// Every recipe in ships/ builds without errors or warnings, stays within a
// phone's triangle budget, and its .glb is well formed.
use ship_core::{gltf, recipe};
use std::fs;

fn budget(class: &str) -> i64 {
    match class {
        "fighter" => 1500,
        "corvette" => 2500,
        "frigate" | "creature" => 4000,
        _ => 9000,
    }
}

#[test]
fn every_ship_builds() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../ships");
    let mut n = 0;
    for e in fs::read_dir(dir).unwrap() {
        let path = e.unwrap().path();
        if !path.to_string_lossy().ends_with(".ship.json") {
            continue;
        }
        let ship = recipe::build(&fs::read_to_string(&path).unwrap());
        assert!(ship.error.is_empty(), "{}: {}", path.display(), ship.error);
        assert!(ship.warnings.is_empty(), "{}: {:?}", path.display(), ship.warnings);
        assert!(ship.mesh.count() > 50, "{}: too few triangles", path.display());
        assert!(ship.mesh.count() <= budget(&ship.class), "{}: {} triangles", path.display(), ship.mesh.count());
        let glb = gltf::write_glb(&ship);
        assert_eq!(&glb[0..4], b"glTF");
        let total = u32::from_le_bytes([glb[8], glb[9], glb[10], glb[11]]) as usize;
        assert_eq!(total, glb.len());
        let jlen = u32::from_le_bytes([glb[12], glb[13], glb[14], glb[15]]) as usize;
        let json = std::str::from_utf8(&glb[20..20 + jlen]).unwrap();
        assert!(json.contains("\"COLOR_0\""));
        n += 1;
    }
    assert!(n >= 5);
}

#[test]
fn floats_round_trip() {
    for &x in &[0.0, 1.0, -2.5, 0.1, 123.456, -0.000123, 6.02e10] {
        let bits = gltf::f32_bits(x) as u32;
        assert_eq!(f32::from_bits(bits), x as f32, "{}", x);
        assert_eq!(gltf::f32_round(x), (x as f32) as f64);
    }
}

#[test]
fn bad_json_is_an_error() {
    let ship = recipe::build("{\"parts\": [");
    assert!(!ship.error.is_empty());
    let ship = recipe::build("{\"parts\": [{\"type\": \"banana\"}]}");
    assert_eq!(ship.warnings.len(), 1);
}
