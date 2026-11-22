// The JavaScript rgrc writes from ship_core builds the same .glb as the
// cargo build: the same JSON chunk byte for byte, and floats equal up to
// 1e-6 (sin / cos / pow may differ in the last bit between Rust's libm and
// JavaScript engines, which shows only in values next to zero).
//   scripts/check-ranger.sh runs it.
const fs = require("fs");
const path = require("path");
const { execFileSync } = require("child_process");
const root = path.join(__dirname, "..");
const m = require(path.join(root, "target/ranger/ship_core.js"));
const dir = path.join(root, "ships");
let bad = 0;
for (const f of fs.readdirSync(dir).filter((f) => f.endsWith(".ship.json")).sort()) {
  const ship = m.Recipe.build(fs.readFileSync(path.join(dir, f), "utf8"));
  const js = Buffer.from(m.Gltf.write_glb(ship));
  const out = path.join(root, "target/parity.glb");
  execFileSync(path.join(root, "target/release/ship"), ["glb", path.join(dir, f), out]);
  const rs = fs.readFileSync(out);
  let ok = js.length === rs.length;
  let worst = 0;
  if (ok) {
    const bin = 28 + js.readUInt32LE(12);
    ok = js.subarray(0, bin).equals(rs.subarray(0, bin));
    for (let i = bin; ok && i < js.length; i += 4) worst = Math.max(worst, Math.abs(js.readFloatLE(i) - rs.readFloatLE(i)));
    ok = ok && worst < 1e-6;
  }
  if (!ok) bad++;
  console.log(`${ok ? "ok  " : "FAIL"} ${f} (${rs.length} bytes, largest float difference ${worst})`);
}
process.exit(bad ? 1 : 0);
