// Regression guard for adding real .log files through the desktop's native picker.
import assert from "node:assert/strict";
import { mockIPC } from "@tauri-apps/api/mocks";

globalThis.window = {};
const calls = [];
const sources = ["C:\\Incoming\\sensor.log", "C:\\Incoming\\raw.bin"];
mockIPC((command, args) => {
  calls.push({ command, args });
  if (command === "plugin:dialog|open") return sources;
  if (command === "workspace_log_import" || command === "run_attach_many") {
    return args.paths.map((source) => ({ source, path: "logs/saved.log", error: null }));
  }
  if (command === "run_file_update") return { files: [], notes: [], logCount: 0, photoCount: 0, fileCount: 0, editable: true };
  throw Error(`Unexpected command: ${command}`);
});
const api = await import("../src/lib/api.ts");
const files = await api.pickFiles("log");
assert.deepEqual(files, sources);
assert.equal(calls[0].command, "plugin:dialog|open");
assert.equal(calls[0].args.options.multiple, true, "logs must support multi-select");
assert.equal(calls[0].args.options.filters, undefined, "binary/custom log names must not be hidden by a CSV-only filter");
await api.workspaceLogImport(files);
assert.deepEqual(calls.at(-1), { command: "workspace_log_import", args: { paths: sources } });
await api.runAttachMany("test-notes", "note-test", "log", files);
assert.deepEqual(calls.at(-1), { command: "run_attach_many", args: { sop: "test-notes", runId: "note-test", kind: "log", paths: sources } });
await api.runFileUpdate("test-notes", "note-test", "logs/saved.log", "预热段", ["复测"]);
assert.deepEqual(calls.at(-1).args.tags, ["复测"]);
assert.equal("syncPull" in api, false);
assert.equal("repoPush" in api, false);
assert.equal("testcasePush" in api, false);
console.log("api: native log picker, multi-file imports, file tags, and no-sync checks pass");
