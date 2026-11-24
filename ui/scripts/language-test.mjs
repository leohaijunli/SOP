// The interface is English; non-English user data remains supported by other tests.
import assert from "node:assert/strict";
import { readdirSync, readFileSync } from "node:fs";
import { dirname, join, relative } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const checked = [];
function walk(folder) {
  for (const entry of readdirSync(folder, { withFileTypes: true })) {
    const path = join(folder, entry.name);
    if (entry.isDirectory()) walk(path);
    else if (/\.(svelte|ts)$/.test(entry.name)) {
      assert(!/\p{Script=Han}/u.test(readFileSync(path, "utf8")), `Non-English interface text in ${relative(root, path)}`);
      checked.push(path);
    }
  }
}
walk(join(root, "src"));
assert.match(readFileSync(join(root, "index.html"), "utf8"), /<html lang="en">/);
console.log(`language: ${checked.length} interface source files checked; document language is English`);
