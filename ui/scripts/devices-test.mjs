// Check the TypeScript `devices` parser against the shared vectors, so it cannot drift
// from the Rust parser that owns the setting. Run with `npm run test:devices`.
import { readFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { formatDevices, parseDevices } from "../src/lib/devices.ts";

const here = dirname(fileURLToPath(import.meta.url));
const vectorsPath = join(here, "..", "..", "crates", "sop-core", "tests", "devices-vectors.json");
const vectors = JSON.parse(readFileSync(vectorsPath, "utf8"));

let failed = 0;
function check(name, actual, expected) {
  const a = JSON.stringify(actual);
  const e = JSON.stringify(expected);
  if (a !== e) {
    failed += 1;
    console.error(`FAIL ${name}\n  expected ${e}\n  actual   ${a}`);
  }
}

for (const [i, vector] of vectors.parse.entries()) {
  check(`parse[${i}] ${JSON.stringify(vector.input)}`, parseDevices(vector.input), vector.devices);
}
for (const [i, vector] of vectors.normalise.entries()) {
  check(
    `normalise[${i}] ${JSON.stringify(vector.input)}`,
    formatDevices(parseDevices(vector.input)),
    vector.output
  );
}

if (failed > 0) {
  console.error(`devices: ${failed} vector(s) failed`);
  process.exit(1);
}
console.log(`devices: all ${vectors.parse.length + vectors.normalise.length} vectors pass`);
