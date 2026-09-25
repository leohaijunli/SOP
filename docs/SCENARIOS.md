# Typical Scenarios

Six situations the tool is built for. Each ends with the design point it exercises.

## 1. Bench calibration of a new magnetometer

**Setting.** Indoor bench, one sensor, no network beyond what the laptop already has. The
operator needs to know whether this unit is fit to take to the field.

**What happens.** The operator picks `mag-sensor-calibration` and starts a run. The app
snapshots the checklist and opens the event log. It walks the 34 steps in order:
conditions first, then power-on and self-test, personnel hygiene, a timed 25-minute
warm-up, static noise windows, an eight-azimuth heading sweep, tilt, an absolute
comparison against a marked reference station, and lever-arm measurements.

At the heading sweep the spread comes out at 12.0 nT, above the 10 nT the procedure
warns about. The procedure does not decide anything. The operator records the eight
readings, marks the step **deviated**, and writes why: suspected mount contribution, not
resolved in this session. The close-out step then asks for a verdict, and it is recorded
as `yes-with-limits` with the limit spelled out: fine for a hand-carried survey, must be
resolved before UAV mounting.

Two log files are attached at the end. The app copies them into the run directory,
hashes them, and commits the record and the logs together.

**Exercises.** Deviation-with-reason as the primary output; recording without judging;
attachments hashed at attach time; a verdict that a human owns.

## 2. A day of ground walk survey, cut short

**Setting.** Three people, nine planned lines, 20 m spacing, base station running.
Overcast, 12 C, wind 3 m/s.

**What happens.** The briefing and line plan are recorded first - the plan is an explicit
step, so the intended nine lines are on record before anyone walks. Then the shared
startup sequence runs: power on, hygiene, site reconnaissance, base station, GNSS,
warm-up, logging write test, and a test line.

The team walks lines 1 to 3. The GPS stays `rtk-fixed` throughout and the base station
shows no gaps at the mid-survey check. At 20:05 the session ends early.

The test line is never repeated, so the operator marks that step **skipped** with the
reason. That single skip is what tells a reviewer a year later that intra-session drift
is unbounded for this dataset. The close-out records three of nine lines, names the six
that were not walked, and leaves the resume point unambiguous for the next visit.

**Exercises.** A skipped step as a bound on validity; explicit session plan for resuming;
mid-survey checks catching slow failures while they are still fixable.

## 3. Adding a new experiment without copying anything

**Setting.** Interference testing is next. It needs the same power-on, the same magnetic
hygiene, the same logging setup, and the same close-out as everything else, plus new
steps for the interference sources.

**What happens.** The new checklist is written as a short file: a few local steps for
source characterisation and shielding checks, plus include markers pointing at the
existing shared procedures. Nothing is copied.

From then on, fixing the hygiene procedure fixes it for calibration, walk survey,
interference, navigation, and UAV survey at once. The experiment matrix stays
maintainable instead of becoming a set of near-identical documents that quietly disagree.

**Exercises.** Composition. This is the single largest difference from a
template-per-experiment design, and the reason the format exists.

## 4. Reusing the same procedures for a UAV-mounted survey

**Setting.** The ground survey procedures are proven. Now the sensor flies.

**What happens.** Most of the checklist is already written: conditions, power-on,
hygiene, warm-up, logging, base station, test line, mid-survey check, pack-down. Only
the platform-specific parts are new - lever arm to the GNSS antenna, separation from
motors and electronics, heading error under the flight mounting, flight lines instead of
walked lines.

The heading error recorded during bench calibration is now the governing number rather
than a footnote: on a moving platform it is the dominant error term, and the calibration
record already says so.

**Exercises.** Reuse across platforms; calibration records carrying forward into the
survey that depends on them.

## 5. A field observation turning into a procedure change

**Setting.** During the walk survey the staff visibly oscillated in the wind. It looks
like a periodic signal at roughly 0.5 Hz and 1-2 nT, and the base station does not show
it.

**What happens.** The observation is written into the run record's notes while it is
fresh. Back at the desk it is copied into `runs/_inbox/` as a short entry: what was seen,
what it should change, which procedure it targets, and a confidence of `medium`, because
the amplitude was estimated from watching the live display rather than measured.

It stays there until it is seen again. Then a pull request updates the `walk-height` step
to add the limiting condition, and deletes the inbox entry. The inbox entry is evidence;
the procedure is the conclusion.

**Exercises.** The knowledge loop. This is the mechanism by which the tool gets better
the longer it is used, and the reason observations are kept separate from conclusions.

## 6. Reviewing a campaign a year later

**Setting.** A dataset is being reprocessed and someone asks whether the sensor was
behaving normally across the campaign.

**What happens.** Each run record is self-contained: the checklist as executed, the
operator, the site, the sensor serial and firmware, the conditions, every deviation with
its reason, and the log files with hashes that prove the analysed file is the collected
file. Nothing depends on the repository history still existing.

Because step ids are authored and permanent, the same step can be compared across runs:
the noise floor from every calibration, the test-line difference from every survey day.
A trend appears that no single run could have shown.

**Exercises.** Stable ids paying off; self-contained records; the audit trail answering
questions nobody thought to ask at the time.

## 7. A new operator's first field day

A master's student joins the campaign and has never used the app or the instrument.

Before the field day they open the app and read `help/overview.md`, then
`help/first-run.md`, then the checklist they are about to run. None of that requires an
internet connection, a manual, or the person who normally runs the survey.

On site they hit their first validation failure - an attached log whose recorded hash no
longer matches the file, because a colleague re-exported it. They press `F1` and find
`help/troubleshooting.md`, which says to find out why the file changed before regenerating
the hash. They do, and it turns out the instrument had written a second, short file over
the first.

Two things came out of that day. The good one is a record that explains itself. The other
is a note in `runs/_inbox/` proposing that the checklist warn about the exporter
overwriting files - which is exactly how the help pages and the procedures are supposed
to improve.
