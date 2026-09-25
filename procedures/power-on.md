---
kind: procedure
procedure_id: power-on
title: Power On and Self-Test
version: 1
updated: 2026-09-24
applies_to: [calibration, ground-survey, interference, navigation, uav-survey]
tags: [power, startup]
---

Bringing the system up and confirming it is in a known, functioning state before any
data is collected.

## Supply voltage

```yaml step
id: poweron-supply
kind: check
severity: critical
captures:
  - key: battery_v
    label: Supply voltage under load
    type: number
    unit: V
    expected:
      min: 11.5
    required: true
  - key: supply_source
    label: Supply source
    type: select
    options: [internal-battery, external-dc, bench-supply]
    required: true
```

Measure under load, not open-circuit. A pack that reads fine unloaded can sag below
the instrument's cutoff a few hours into a survey, which shows up as a recording gap
rather than a clean shutdown.

Charge or swap before starting if the pack is below the level needed for the planned
session, including the trip back.

- [ ] Voltage measured with the instrument powered on
- [ ] Voltage is sufficient for the planned session plus reserve

## Power on and boot

```yaml step
id: poweron-boot
kind: check
severity: normal
captures:
  - key: firmware_version
    label: Firmware version reported at boot
    type: text
    required: true
```

Record the firmware version as reported by the instrument, not as printed on a label.
Calibration results are only comparable within a firmware version.

- [ ] Instrument powers on
- [ ] Display or console output is updating
- [ ] Firmware version recorded

## Instrument self-test

```yaml step
id: poweron-self-test
kind: check
severity: critical
captures:
  - key: self_test_result
    label: Self-test result as reported
    type: select
    options: [ok, warning, error]
    expected: ok
    required: true
```

Record what the instrument says. A `warning` or `error` here is a decision point for
the operator, not something the tooling interprets: either resolve it, or record why
you are proceeding anyway as a deviation.

- [ ] Self-test run to completion
- [ ] Result recorded verbatim

## Time base

```yaml step
id: poweron-clock
kind: check
severity: critical
captures:
  - key: clock_offset_s
    label: Offset from UTC reference
    type: number
    unit: s
    required: true
```

Compare the instrument clock against GNSS or a synchronised reference. Timestamps in
the instrument log are only useful for merging with base-station and GNSS data if this
offset is known; a clock that is minutes out silently corrupts diurnal correction.

- [ ] Clock compared against a UTC reference
- [ ] Offset recorded, even if it is zero
