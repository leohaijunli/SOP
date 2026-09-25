---
created: 2026-09-24
author: leo
observed_in: runs/ground-walk-survey/2026-09-24-renfrew-walk01.md
target: procedures/walk-line.md
confidence: medium
---

At a measured wind speed of about 3 m/s the staff showed a visible oscillation, which
would appear in the data as a periodic signal at roughly 0.5 Hz with an amplitude of
1 to 2 nT. The base station showed no matching oscillation, so it is the staff moving
and not the field.

If this holds up, it belongs in the `walk-height` step of `walk-line.md` as a limiting
condition: above some wind speed the sensor must be held by a rigid harness rather
than a hand-carried staff, or the affected lines must be flagged so the oscillation
can be filtered in processing.

Observed once so far. The frequency and amplitude are estimates from watching the live
display, not from a measured spectrum. Worth repeating with the log running before
this becomes a procedural change.
