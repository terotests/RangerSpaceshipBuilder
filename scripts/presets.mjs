// The recipes in a folder as one script for the editor:
//   node scripts/presets.mjs ships dist/web/presets.js
// Order: by "order" in the recipe, then by length.
import fs from "node:fs";
import path from "node:path";
const [dir, out] = process.argv.slice(2);
const list = fs.readdirSync(dir).filter((f) => f.endsWith(".ship.json")).map((f) => {
  const recipe = JSON.parse(fs.readFileSync(path.join(dir, f), "utf8"));
  return { id: f.replace(/\.ship\.json$/, ""), recipe };
});
list.sort((a, b) => (a.recipe.order ?? 0) - (b.recipe.order ?? 0) || (a.recipe.length ?? 0) - (b.recipe.length ?? 0));
fs.writeFileSync(out, "window.SHIP_PRESETS = " + JSON.stringify(list) + ";\n");
console.log(`${list.length} presets -> ${out}`);
