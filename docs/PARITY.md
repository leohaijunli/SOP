# Parity with the Python tooling

The Python tooling under `tools/` was the reference implementation of the format while
the Rust one was written. `sop validate` and `sop index` are now ports of it, and this
file records the evidence and the deliberate differences. It is deleted with `tools/`
(see `docs/DECISIONS.md` D11).

## Evidence

Measured on the repository as it stands, with all 23 procedures, checklists, run records,
and help pages in place:

| Check | Python | Rust |
|---|---|---|
| Content validation | `0 error(s), 0 warning(s)` | `0 error(s), 0 warning(s)` |
| Negative test cases | 22 of 22 | 22 of 22, plus 3 warning cases |
| Checkbox/selftest control | clean copy validates | clean copy validates |
| Manifest | `dist/manifest.json` | identical except as listed below |

Both negative suites break exactly one thing at a time and require the validator to fail
*for the right reason*, so neither can pass by rejecting everything.

## Deliberate differences

**YAML 1.1 versus YAML 1.2.** Python's `yaml.safe_load` resolves `yes`, `no`, `on`, and
`off` as booleans; `serde_norway` follows the YAML 1.2 core schema and keeps them as
strings. This is not a bug in either: it is a version difference, and it produced a real
one. `checklists/mag-sensor-calibration.md` declared

```yaml
options: [yes, yes-with-limits, no]
```

so the Python manifest contained `true` and `false` where the author had written `yes`
and `no`, and the app would have offered the wrong choices. The content now quotes them,
and `SPEC.md` section 3 says to quote scalars like these.

**Timestamps are passed through, not re-serialised.** Python parsed
`started: 2026-09-22T18:30:00Z` into a `datetime` and wrote `2026-09-22T18:30:00+00:00`
back. Rust keeps the author's text. Both are valid ISO 8601; the Rust form means the
generated manifest contains what is in the file, which makes the diff easier to read.
This is the only remaining difference in `dist/manifest.json`.

**Diagnostics keep their line numbers.** Python reports cross-file step id collisions
without a line; the Rust version reports the line of the second definition. Python also
reports a duplicate step id inside one file twice, once from the per-file check and once
from the resolution check; Rust reports it once.

**A missing `applies_to` produces one error, not two.** Python reports both "missing
required key" and "must list at least one experiment type"; Rust reports the first.

**Non-string `options` are rejected.** Rust requires a `select` capture's options to be
strings, because options are rendered as choices and compared as text. Python never
checked the element types. Adding this check is what makes a typo like an unquoted `no`
visible rather than silent.

**An unknown capture type stays a hard error**, exactly as `SPEC-COMPAT.md` requires.

**`project.md` is checked by Rust and not by Python.** It was added after the port, so
`tools/validate.py` does not know the file exists and reports a clean run on a repository
whose project file is broken. `SPEC.md` section 14 is the specification; `sop validate` is
the implementation that keeps up with it. The Python tooling is deleted with `tools/`
(D11), and this difference disappears with it.

## Known gaps, carried over rather than fixed during the port

These are behaviours the port reproduces on purpose. Changing behaviour while porting
makes it impossible to tell a port bug from an intended change.

- The content schema declared by a run record is not checked. `schema` is only inspected
  for procedures and checklists, although `SPEC.md` section 13 does not say it should be.
- `expected` on a `bool` capture is accepted without checking that it is a boolean, if
  the value arrived as a string.

Fix these deliberately, with a test, not as a side effect of something else.
