---
kind: run
run_id: 2026-09-29-heading-error-uas-mag
sop: heading-error
sop_version: 1
sop_commit: b74cea4
operator: leo
site: li
plan: uas-mag-preflight
case: heading-error
started: 2026-09-29T12:56:09Z
ended: 2026-09-29T12:56:56Z
status: complete
sensor:
  model: UAS-MAG
hardware:
  - Bartington UAS-MAG fluxgate
  - Non-magnetic rotation table
  - Azimuth marks
added_steps: 1
deviations_count: 0
---

# Run heading-error (2026-09-29T12:56:09Z)

operator: leo

site: li

plan: uas-mag-preflight
case: heading-error

sop_version: 1
sop_commit: b74cea4
sensor: UAS-MAG
hardware: Bartington UAS-MAG fluxgate, Non-magnetic rotation table, Azimuth marks

## Timeline

| Step | Title | Start (UTC) | End (UTC) | Status | Duration |
|------|-------|-------------|-----------|--------|----------|
| mount | Mount | 12:56:09Z | 12:56:38Z | done | 29s |
| rotate | Rotate | 12:56:38Z | 12:56:40Z | done | 2s |
| adhoc-000 | test | 12:56:40Z | 12:56:43Z | done | 3s |
| result | Result | 12:56:43Z | 12:56:46Z | done | 3s |

## Mount

- [ ] Sensor centered on the rotation table
- [ ] Table marked at known azimuths
- [ ] No nearby ferrous objects

```yaml result
step: mount
status: done
opened_at: "2026-09-29T12:56:09Z"
ended_at: "2026-09-29T12:56:38Z"
```

## Rotate

- [ ] Full 360-degree sweep recorded
- [ ] Azimuth and reading logged at each mark
- [ ] Peak-to-peak heading error computed

```yaml result
step: rotate
status: done
opened_at: "2026-09-29T12:56:38Z"
ended_at: "2026-09-29T12:56:40Z"
```

## Result

- [ ] Heading error logged
- [ ] Residual error acknowledged by the operator

```yaml result
step: result
status: done
opened_at: "2026-09-29T12:56:43Z"
ended_at: "2026-09-29T12:56:46Z"
```

## Steps added during the run

| ID | 标题 | 插入位置 | 状态 | 开始 | 结束 |
|----|------|----------|------|------|------|
| adhoc-000 | test | after rotate | done | 12:56:40Z | 12:56:43Z |


status: complete
deviations_count: 0
