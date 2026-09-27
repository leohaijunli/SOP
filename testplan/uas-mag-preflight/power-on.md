---
kind: checklist
sop_id: uas-mag-preflight-power-on
title: Power On and Boot
order: 10
version: 1
status: active
equipment:
  - Bartington UAS-MAG fluxgate
  - Flight battery
  - Field laptop
---

Bring the Bartington UAS-MAG up and confirm it boots into a known, stable state.

## Supply

- [ ] Supply within the sensor's operating range
- [ ] No brown-out during boot
- [ ] Record the supply voltage
- [ ] Note the power source (flight battery / bench supply / external DC)

## Firmware

- [ ] Firmware version logged for the record
- [ ] No error codes on boot

## Self-test

- [ ] Sensor self-test completed
- [ ] Data stream present at the configured rate
- [ ] Self-test marked as passed