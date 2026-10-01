# Run uas-mag-preflight-power-on (2026-09-30T23:56:45Z)

operator: Leo

site: cfar

plan: uas-mag-preflight
case: power-on

sop_version: 1
sop_commit: f93bc59
sensor: UAS-MAG
hardware: Bartington UAS-MAG fluxgate, Flight battery, Field laptop, UAS-MAG

## Timeline

| Step | Title | Start (UTC) | End (UTC) | Status | Duration |
|------|-------|-------------|-----------|--------|----------|
| supply | Supply | 00:26:27Z | — | open | — |
| firmware | Firmware | — | — | open | — |
| self-test | Self-test | — | — | open | — |

## Supply

- [ ] Supply within the sensor's operating range
- [ ] No brown-out during boot
- [ ] Record the supply voltage
- [ ] Note the power source (flight battery / bench supply / external DC)

> attachment (log): runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag-2/logs/heading-error.md (sha256 3bcdcd5cc29a, 647 bytes)

## Firmware

- [ ] Firmware version logged for the record
- [ ] No error codes on boot

## Self-test

- [ ] Sensor self-test completed
- [ ] Data stream present at the configured rate
- [ ] Self-test marked as passed

deviations_count: 0
