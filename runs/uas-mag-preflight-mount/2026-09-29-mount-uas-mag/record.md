# Run uas-mag-preflight-mount (2026-09-29T13:08:06Z)

operator: leo

site: cfar

plan: uas-mag-preflight
case: mount

sop_version: 1
sop_commit: b74cea4
sensor: UAS-MAG
hardware: Bartington UAS-MAG fluxgate, UAV with payload mount, Non-magnetic fasteners

## Timeline

| Step | Title | Start (UTC) | End (UTC) | Status | Duration |
|------|-------|-------------|-----------|--------|----------|
| mounting | Mounting | 13:08:06Z | 13:08:07Z | done | 1s |
| orientation | Orientation | 13:08:07Z | 13:08:09Z | done | 2s |

## Mounting

- [ ] Sensor fixed and cannot rotate in flight
- [ ] No ferrous fasteners within the sensor's range
- [ ] Mounted away from motors and battery leads
- [ ] Mount offset recorded

```yaml result
step: mounting
status: done
opened_at: "2026-09-29T13:08:06Z"
ended_at: "2026-09-29T13:08:07Z"
```

## Orientation

- [ ] Axis aligned with the flight frame
- [ ] Roll, pitch, and yaw recorded against the UAV body frame
- [ ] Orientation angles within tolerance

```yaml result
step: orientation
status: done
opened_at: "2026-09-29T13:08:07Z"
ended_at: "2026-09-29T13:08:09Z"
```


status: complete
deviations_count: 0
