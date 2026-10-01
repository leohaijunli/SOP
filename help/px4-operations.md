---
kind: help
help_id: px4-operations
title: PX4 Operations
section: PX4 Operations
order: 10
audience: operator
updated: 2026-09-29
summary: Flight controller, payload, and survey-flight procedures for PX4-based aircraft.
---

Help and reference for operating PX4-based aircraft during a magnetic survey.

## Before you fly

- Confirm the autopilot is on a supported firmware and the sensor is not mounted near
  moving ferrous parts.
- Confirm the magnetometer is calibrated and its offsets are current.
- Set the RTK base logging before arming so every fix can be traced back to a reference.

## Flight deck

- Arm and disarm through the ground control station, never from the aircraft.
- Fly the planned lines at constant height and speed so the coverage log stays clean.
- Watch for gaps longer than one line; they are easier to fix by reflying than by
  processing later.

## After landing

- Copy the payload log and the flight log to the field laptop with the same run id.
- Add the payload log to the run as a `log` and any photographs of the site as `photo`
  files, from the History detail drawer or `sop run attach --kind`. Both are allowed
  after the run has ended.
- Checksum the raw magnetometer data before leaving the field.

See the individual pages under this category for the step-by-step details.
