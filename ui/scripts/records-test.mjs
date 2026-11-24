import assert from "node:assert/strict";
import { matchesQuery, parseTags, fileMatches, sortFiles, hasTag, tagChoices, collectFiles, recordMatches } from "../src/lib/records.ts";

assert.deepEqual(parseTags("温漂， GPS ,gps;复测； ;温漂"), ["温漂", "GPS", "复测"]);
assert(matchesQuery("预热 GPS", ["GPS 延迟，预热后再采样"]));
assert(!matchesQuery("预热 缺失", ["预热后再采样"]));
assert(matchesQuery("ＲＭ３１００", ["rm3100"]));

const row = (key, time, added = time) => ({ key, run: null, label: "温度测试", editable: true,
  file: { name: `${key}.csv`, path: `logs/${key}.csv`, fileTime: time, addedAt: added,
    tags: ["温漂", "GPS"], remark: "预热 10 秒", kind: "log", size: 1, missing: false } });
const files = [row("b", "2026-11-24T10:00:00Z", "2026-11-24T13:00:00Z"),
  row("a", "2026-11-24T09:00:00Z", "2026-11-24T14:00:00Z"), row("c", "unknown")];
assert.deepEqual(sortFiles(files, "newest", "file").map((file) => file.key), ["b", "a", "c"]);
assert.deepEqual(sortFiles(files, "oldest", "file").map((file) => file.key), ["a", "b", "c"]);
assert.deepEqual(sortFiles(files, "newest", "added").map((file) => file.key), ["a", "b", "c"]);
assert.deepEqual(files.map((file) => file.key), ["b", "a", "c"], "sorting must not mutate the manifest");
assert(fileMatches(files[0], "GPS 预热"));
assert(hasTag(files[0].file, "gps"));
assert.deepEqual(tagChoices(files).sort(), ["GPS", "温漂"].sort());
const run = { sop: "test-notes", run_id: "one", title: "基线采样", editable: true,
  notes: [{ text: "电源干扰明显" }], files: [files[0].file, files[0].file] };
const manifest = { runs: [run], workspace_files: [files[1].file], testplans: [], checklists: [] };
assert.equal(collectFiles(manifest).length, 2, "one physical file in one record is listed once");
assert(recordMatches(run, manifest, "电源 gps"), "search must include notes and file tags together");
console.log("records: tag, full-text search, chronological order, and deduplication checks pass");
