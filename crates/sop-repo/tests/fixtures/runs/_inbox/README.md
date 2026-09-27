# Inbox

Observations that look worth turning into procedure changes, but that have not been
proven yet.

This directory is **never executed**. It holds no checklists and no run records. It
exists so that "we should add this to the SOP" has somewhere to live other than
someone's memory at the end of a long day.

## How to use it

1. During a run, note the observation in the run record.
2. At review time, if it looks reusable, write a short file here.
3. When it has held up more than once, open a pull request that edits the relevant
   procedure and deletes the inbox file.

Keeping the inbox file and the procedural change separate is deliberate: the inbox
entry is evidence, and the procedure is the conclusion drawn from it.

## Entry format

```markdown
---
created: 2026-09-24
author: Name
observed_in: runs/ground-walk-survey/2026-09-24-renfrew-walk01.md
target: procedures/walk-line.md
confidence: low | medium | high
---

What was observed, in one or two sentences.

Why it matters, and what it should change.

How many times it has now been observed.
```

Small and specific beats long and speculative. "Wind above roughly 5 m/s makes the
staff unsteady, which shows up as a 2 nT oscillation at 0.5 Hz" is useful; "the
survey was harder than expected" is not.

## Removal

An inbox file is deleted when its change lands in a procedure, or when it is decided
not to act on it. Either outcome is fine; leaving it to accumulate indefinitely is not.
