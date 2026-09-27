# Compatibility Policy

Two schemas exist and are versioned separately:

| Schema | Carried by | Defined in |
|---|---|---|
| Content | `schema:` in file front matter, default 1 | `SPEC.md` |
| Event log | `LogMeta.schema`, first line of `events.jsonl` | `docs/DESIGN.md` section 6.3 |

## Starting point

The format has not shipped and has no external consumers. This policy takes effect at
schema **1**, which is the vocabulary as it stands after the P0-a additions in
`SPEC.md`: `attach` as a capture type, `expected` on captures, and the checkbox
semantics rule.

There is therefore no compatibility obligation to any earlier draft of this repository.
Files that predate the policy simply omit `schema` and are read as schema 1.

## What does not bump the schema

A change is additive if a reader that predates it can still handle the file **without
corrupting or misreading anything**. Additive changes do not bump the version:

- a new optional front-matter key;
- a new optional key inside a `yaml step` block;
- a new value in a controlled vocabulary that is not a capture type;
- a new event type in the log, provided replay of existing logs is unaffected.

Readers must therefore:

- **warn and ignore** an unknown front-matter key, an unknown step key, or an unknown
  capture key, and must preserve them verbatim when writing the file back;
- never assume a key list is exhaustive.

This is what makes additive evolution safe. A file using a key we have not invented yet
still opens, still validates, and still round-trips through the writer unchanged.

## What bumps the schema

A change is breaking if an older reader would silently do the wrong thing, or would
fail in a way that is not obvious:

- adding or removing a **capture type**, or changing what one means;
- changing the meaning of an existing key;
- removing or renaming a required key;
- changing id rules, or the storage layout;
- changing how include markers resolve, or the step ordering they produce;
- changing the event log's replay semantics.

Adding a capture type is listed as breaking on purpose, because an unknown capture type
is a hard error rather than a warning. Treating it as `text` would be exactly the silent
degradation this policy exists to prevent - and a typo in a capture type would then go
unnoticed forever. The cost is that a genuinely new capture type needs a version bump.
That cost is small and it is paid rarely.

## No forward compatibility

A reader that meets a file or log declaring a schema newer than it supports **rejects it
with a clear message** naming the required version. It does not guess, does not keep an
"unknown" fallback path, and does not attempt a two-pass parse.

Rationale: this is a desktop application plus a CI job, so upgrading is cheap and
immediate. Forward-compatible parsing costs real complexity - preserving unknown
constructs, tracking indices for reversals, rendering things it does not understand -
and buys protection against a situation that is easily avoided by upgrading. Guessing
at a format is more dangerous than refusing it.

## Deprecation

- A step or capture is retired by setting `deprecated: true`. Deprecated items stop
  being shown to operators but remain valid, so historical run records still resolve.
- **Ids are never reused and never renumbered.** A run record cites step ids; reusing an
  id silently reattributes old results to a new step. This is the one mistake the format
  cannot detect after the fact, so it is stated here as a rule rather than a check.
- Removal happens only at a schema bump.

## Event log rules

- The log is the source of truth for a run, so its rules are stricter than the content
  rules. An unknown event type is a **hard error**: a skipped event means the replayed
  state is wrong, and wrong state is worse than a refusal to open.
- Within a schema version, replay must be deterministic and must not depend on the tool
  version. Given the same log, the same state is derived.
- Reversal is modelled as a new typed event, never as an edit or deletion. See
  `docs/DESIGN.md` section 6.2.

## How to make a breaking change

1. Bump the schema number in `SPEC.md` and here.
2. Write the migration note below.
3. Let the reader reject older/newer files with a message that names the version.
4. Migrate the content in this repository in the same pull request, so `main` is never
   left holding files its own validator rejects.

Additive changes need none of this. They need only a line in the changelog below.

## Changelog

| Version | Date | Change |
|---|---|---|
| 1 | 2026-09-24 | Baseline. Content schema, capture types including `attach`, `expected` on captures, checkbox semantics, one-run-one-directory storage. |
| 1 | 2026-09-25 | Additive: `help/*.md` as a fourth file kind for in-app documentation. An older reader does not look in `help/`, so nothing is misread. |
| 1 | 2026-09-25 | Additive: `project.md` as the repository's identity. A reader that does not know the file ignores it, and `sop project` only ever rewrites one front-matter key, so unknown keys survive an edit. |
| 1 | 2026-09-26 | Additive: optional `sensor`, `hardware`, and `conditions` on a run record, and the matching optional fields on `RunStarted`. A reader that predates them warns about an unknown front-matter key and replays a log with unknown event fields untouched, and the record only gains keys a reader may ignore. |
| 1 | 2026-09-26 | Additive: a `yaml result` block may omit `status`, recording that a step happened with no outcome. The vocabulary is unchanged, and a reader that requires the key fails on the file openly rather than misreading it as `done`. |
