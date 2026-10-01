---
kind: run
run_id: 2026-09-30-power-on-uas-mag
sop: uas-mag-preflight-power-on
sop_version: 1
sop_commit: f93bc59
operator: Leo
site: cfar
plan: uas-mag-preflight
case: power-on
started: 2026-09-30T23:48:08Z
ended: 2026-09-30T23:56:32Z
status: complete
sensor:
  model: UAS-MAG
hardware:
  - Bartington UAS-MAG fluxgate
  - Flight battery
  - Field laptop
  - UAS-MAG
logs:
  - path: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/logs/heading-error.md
    sha256: 3bcdcd5cc29a85a2e26a1adfd76214bf82d3acd892d073e51410ba94658d146f
    size: 647
  - path: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/photos/screenshot-2026-09-07_18-14-32.png
    kind: photo
    sha256: db7105d34e4ded136d4005cf41cf47360c6529aa26a8df6336f9ce6988fee6ac
    size: 57117
events_sealed_bytes: 1120
events_sha256: 6dcf8809bf2e78a18713fc1f37003eb2ab02c59f46c279896544e5b9ef06dbb6
deviations_count: 0
---

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
- photo: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/photos/screenshot-2026-09-07_18-14-32.png (sha256 db7105d34e4d)

## Added after the run ended

_Added after the run ended; the field record above is unchanged._

- 2026-10-01T01:09:53Z — log: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/logs/heading-error.md (647 bytes, sha256 3bcdcd5cc29a)
- 2026-10-01T01:13:18Z — photo: runs/uas-mag-preflight-power-on/2026-09-30-power-on-uas-mag/photos/screenshot-2026-09-07_18-14-32.png (57117 bytes, sha256 db7105d34e4d)


status: complete
deviations_count: 0
