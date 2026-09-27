---
kind: checklist
sop_id: uas-mag-preflight-gnss
title: GNSS Lock and Time Sync
order: 30
version: 1
status: active
equipment:
  - RTK GNSS base and rover
  - UAS-MAG fluxgate
  - Field laptop
---

Confirm the sensor is position- and time-tagged correctly before the payload logs start.

## Satellite fix

- [ ] Fixed or float fix available
- [ ] Enough satellites for the planned accuracy
- [ ] Record the fix type and satellite count

## Time sync

- [ ] Sensor clock synced to GNSS time
- [ ] Clock offset within the survey's timing tolerance
- [ ] Offset recorded in milliseconds