# Run uas-mag-preflight-gnss (2026-09-29T13:08:16Z)

operator: leo

site: cfar

plan: uas-mag-preflight
case: gnss-lock

sop_version: 1
sop_commit: b74cea4
sensor: UAS-MAG
hardware: RTK GNSS base and rover, UAS-MAG fluxgate, Field laptop

## Timeline

| Step | Title | Start (UTC) | End (UTC) | Status | Duration |
|------|-------|-------------|-----------|--------|----------|
| satellite-fix--2 | Satellite fix  2 | 13:08:16Z | 13:08:18Z | done | 2s |
| time-sync | Time sync | 13:08:18Z | 13:08:18Z | done | 0s |

## Satellite fix  2

- [ ] Fixed or float fix available
- [ ] Enough satellites for the planned accuracy
- [ ] Record the fix type and satellite count

```yaml result
step: satellite-fix--2
status: done
opened_at: "2026-09-29T13:08:16Z"
ended_at: "2026-09-29T13:08:18Z"
```

## Time sync

- [ ] Sensor clock synced to GNSS time
- [ ] Clock offset within the survey's timing tolerance
- [ ] Offset recorded in milliseconds

```yaml result
step: time-sync
status: done
opened_at: "2026-09-29T13:08:18Z"
ended_at: "2026-09-29T13:08:18Z"
```


status: complete
deviations_count: 0
