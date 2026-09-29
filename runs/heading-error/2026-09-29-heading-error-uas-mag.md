---
kind: run
run_id: 2026-09-29-heading-error-uas-mag
sop: heading-error
sop_version: 1
sop_commit: b74cea4
operator: leo
site: cfar
plan: uas-mag-preflight
case: heading-error
started: 2026-09-29T13:07:55Z
ended: 2026-09-29T13:08:03Z
status: complete
sensor:
  model: UAS-MAG
hardware:
  - Bartington UAS-MAG fluxgate
  - Non-magnetic rotation table
  - Azimuth marks
deviations_count: 0
---

# Run heading-error (2026-09-29T13:07:55Z)

operator: leo

site: cfar

plan: uas-mag-preflight
case: heading-error

sop_version: 1
sop_commit: b74cea4
sensor: UAS-MAG
hardware: Bartington UAS-MAG fluxgate, Non-magnetic rotation table, Azimuth marks

## Timeline

| Step | Title | Start (UTC) | End (UTC) | Status | Duration |
|------|-------|-------------|-----------|--------|----------|
| mount | Mount | 13:07:55Z | 13:07:57Z | done | 2s |
| rotate | Rotate | 13:07:57Z | 13:07:57Z | done | 0s |
| result | Result | 13:07:57Z | 13:07:59Z | done | 2s |

## Mount

- [ ] Sensor centered on the rotation table
- [ ] Table marked at known azimuths
- [ ] No nearby ferrous objects

```yaml result
step: mount
status: done
opened_at: "2026-09-29T13:07:55Z"
ended_at: "2026-09-29T13:07:57Z"
```

## Rotate

- [ ] Full 360-degree sweep recorded
- [ ] Azimuth and reading logged at each mark
- [ ] Peak-to-peak heading error computed

```yaml result
step: rotate
status: done
opened_at: "2026-09-29T13:07:57Z"
ended_at: "2026-09-29T13:07:57Z"
```

## Result

- [ ] Heading error logged
- [ ] Residual error acknowledged by the operator

```yaml result
step: result
status: done
opened_at: "2026-09-29T13:07:57Z"
ended_at: "2026-09-29T13:07:59Z"
```


status: complete
deviations_count: 0
