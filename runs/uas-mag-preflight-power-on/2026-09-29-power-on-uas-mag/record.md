# Run uas-mag-preflight-power-on (2026-09-29T13:07:36Z)

operator: leo

site: cfar

plan: uas-mag-preflight
case: power-on

sop_version: 1
sop_commit: b74cea4
sensor: UAS-MAG
hardware: Bartington UAS-MAG fluxgate, Flight battery, Field laptop

## Timeline

| Step | Title | Start (UTC) | End (UTC) | Status | Duration |
|------|-------|-------------|-----------|--------|----------|
| supply | Supply | 13:07:36Z | 13:07:38Z | done | 2s |
| firmware | Firmware | 13:07:38Z | 13:07:39Z | done | 1s |
| self-test | Self-test | 13:07:39Z | 13:07:40Z | done | 1s |

## Supply

- [ ] Supply within the sensor's operating range
- [ ] No brown-out during boot
- [ ] Record the supply voltage
- [ ] Note the power source (flight battery / bench supply / external DC)

```yaml result
step: supply
status: done
opened_at: "2026-09-29T13:07:36Z"
ended_at: "2026-09-29T13:07:38Z"
```

## Firmware

- [ ] Firmware version logged for the record
- [ ] No error codes on boot

```yaml result
step: firmware
status: done
opened_at: "2026-09-29T13:07:38Z"
ended_at: "2026-09-29T13:07:39Z"
```

## Self-test

- [ ] Sensor self-test completed
- [ ] Data stream present at the configured rate
- [ ] Self-test marked as passed

```yaml result
step: self-test
status: done
opened_at: "2026-09-29T13:07:39Z"
ended_at: "2026-09-29T13:07:40Z"
```


status: complete
deviations_count: 0
