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
| supply | Supply | 01:38:30Z | 01:38:31Z | done | 1s |
| firmware | Firmware | 01:38:31Z | 01:38:33Z | done | 2s |
| self-test | Self-test | 01:38:33Z | 01:38:35Z | done | 2s |

## Supply

- [ ] Supply within the sensor's operating range
- [ ] No brown-out during boot
- [ ] Record the supply voltage
- [ ] Note the power source (flight battery / bench supply / external DC)

```yaml result
step: supply
status: done
opened_at: "2026-10-01T01:38:30Z"
ended_at: "2026-10-01T01:38:31Z"
```

> attachment (log): runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag-2/logs/heading-error.md (sha256 3bcdcd5cc29a, 647 bytes)

## Firmware

- [ ] Firmware version logged for the record
- [ ] No error codes on boot

```yaml result
step: firmware
status: done
opened_at: "2026-10-01T01:38:31Z"
ended_at: "2026-10-01T01:38:33Z"
```

## Self-test

- [ ] Sensor self-test completed
- [ ] Data stream present at the configured rate
- [ ] Self-test marked as passed

```yaml result
step: self-test
status: done
opened_at: "2026-10-01T01:38:33Z"
ended_at: "2026-10-01T01:38:35Z"
```


status: complete
deviations_count: 0
