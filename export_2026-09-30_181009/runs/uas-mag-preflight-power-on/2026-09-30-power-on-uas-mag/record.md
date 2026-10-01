# Run uas-mag-preflight-power-on (2026-09-30T23:48:08Z)

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
| supply | Supply | 23:56:24Z | 23:56:26Z | done | 2s |
| firmware | Firmware | 23:56:26Z | 23:56:27Z | done | 1s |
| self-test | Self-test | 23:56:27Z | 23:56:28Z | done | 1s |

## Supply

- [ ] Supply within the sensor's operating range
- [ ] No brown-out during boot
- [ ] Record the supply voltage
- [ ] Note the power source (flight battery / bench supply / external DC)

```yaml result
step: supply
status: done
opened_at: "2026-09-30T23:56:24Z"
ended_at: "2026-09-30T23:56:26Z"
```

## Firmware

- [ ] Firmware version logged for the record
- [ ] No error codes on boot

```yaml result
step: firmware
status: done
opened_at: "2026-09-30T23:56:26Z"
ended_at: "2026-09-30T23:56:27Z"
```

## Self-test

- [ ] Sensor self-test completed
- [ ] Data stream present at the configured rate
- [ ] Self-test marked as passed

```yaml result
step: self-test
status: done
opened_at: "2026-09-30T23:56:27Z"
ended_at: "2026-09-30T23:56:28Z"
```

## Run attachments

- log: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/logs/heading-error.md (sha256 3bcdcd5cc29a)

## Added after the run ended

_Added after the run ended; the field record above is unchanged._

- 2026-10-01T01:09:53Z — log: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/logs/heading-error.md (647 bytes, sha256 3bcdcd5cc29a)


status: complete
deviations_count: 0
