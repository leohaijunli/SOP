---
kind: run
run_id: 2026-09-24-renfrew-walk01
sop: ground-walk-survey
sop_version: 1
sop_commit: unknown
operator: leo
site: Renfrew 395
started: 2026-09-24T16:00:00Z
ended: 2026-09-24T20:05:00Z
status: partial
sensor:
  model: GEM GSM-19
  serial: "4451233"
  firmware: "7.0"
hardware:
  - mag_gcs v0.3.1
  - GNSS receiver, RTK
conditions:
  weather: overcast
  temp_c: 12
logs:
  - path: logs/2026-09-24-renfrew-walk01/mag_raw.csv
    sha256: a8aa8a242ca9973c717793164adea40c26431c51cdc978fcaa902e45957136b5
    size: 18623
    description: Survey magnetometer log
  - path: logs/2026-09-24-renfrew-walk01/base_station.csv
    sha256: c8ec82022b5eee1145d1b9b479d27afc7e4595b641927a018a356e4841ab0048
    size: 27923
    description: Base station log for diurnal correction
deviations_count: 0
---

Fixture run to exercise the checklist end to end. Both attached logs are short
synthetic samples, not real survey data.

Session ended at 20:05 rather than at the planned end of daylight. Lines 4 through 9
were not walked; the base station was left logging until pack-down.

## cond-location

```yaml result
step: cond-location
status: done
captures:
  session_location: Renfrew 395, north field
  nearby_sources: Distribution line along the east boundary, approx 180 m
```

## cond-environment

```yaml result
step: cond-environment
status: done
captures:
  ambient_c: 12
  wind_mps: 3
  humidity_pct: 72
```

## cond-reference-instrument

```yaml result
step: cond-reference-instrument
status: done
captures:
  reference_instrument: none
  reference_last_calibrated: n/a
```

## walk-briefing

```yaml result
step: walk-briefing
status: done
captures:
  team_size: 3
  roles: leo operator and record; second walker on GNSS; third on base station
```

## walk-line-plan

```yaml result
step: walk-line-plan
status: done
captures:
  lines_planned: 9
  lines_completed: 3
  session_goal: Walk lines 1 to 9, 20 m spacing, north-south
```

## poweron-supply

```yaml result
step: poweron-supply
status: done
captures:
  battery_v: 12.4
  supply_source: internal-battery
```

## poweron-boot

```yaml result
step: poweron-boot
status: done
captures:
  firmware_version: "7.0"
```

## poweron-self-test

```yaml result
step: poweron-self-test
status: done
captures:
  self_test_result: ok
```

## poweron-clock

```yaml result
step: poweron-clock
status: done
captures:
  clock_offset_s: 0.0
```

## hygiene-person

```yaml result
step: hygiene-person
status: done
```

## hygiene-equipment

```yaml result
step: hygiene-equipment
status: done
```

## hygiene-separation

```yaml result
step: hygiene-separation
status: done
captures:
  nearest_vehicle_m: 120
  nearest_structure_m: 180
```

## hygiene-record

```yaml result
step: hygiene-record
status: done
```

Vehicle remained parked at the gate, 120 m from the nearest line. No movement during
the survey.

## recon-access

```yaml result
step: recon-access
status: done
captures:
  landowner_contact: Site contact, mobile, on file
  access_notes: Gate at the north end, unlocked
```

## recon-hazards

```yaml result
step: recon-hazards
status: done
captures:
  hazards: Distribution line along the east boundary; two fence lines; wet ground at the south end
```

## recon-cleanliness

```yaml result
step: recon-cleanliness
status: done
captures:
  worst_gradient_nt: 38
  worst_gradient_location: Line 6, midpoint, near the buried field drain
```

## recon-grid

```yaml result
step: recon-grid
status: done
captures:
  line_spacing_m: 20
  station_spacing_m: 0.5
  line_azimuth_deg: 0
  tie_line_spacing_m: 100
```

## recon-reference

```yaml result
step: recon-reference
status: done
captures:
  reference_station_id: REF-395-A
```

## base-site

```yaml result
step: base-site
status: done
captures:
  base_distance_to_road_m: 300
  base_lat_deg: 48.5694
  base_lon_deg: -123.6291
```

## base-install

```yaml result
step: base-install
status: done
captures:
  sensor_height_m: 1.0
```

## base-logging

```yaml result
step: base-logging
status: done
captures:
  base_sample_rate_hz: 1
  base_start_time: 2026-09-24T15:45:00Z
```

Started 15 minutes before the first line.

## base-verify

```yaml result
step: base-verify
status: done
captures:
  first_min_sigma_nt: 0.06
```

## gnss-datum

```yaml result
step: gnss-datum
status: done
captures:
  datum: WGS84
  height_reference: ellipsoidal
```

## gnss-fix

```yaml result
step: gnss-fix
status: done
captures:
  fix_type: rtk-fixed
  satellites: 18
  hdop: 0.7
```

## gnss-height

```yaml result
step: gnss-height
status: done
captures:
  antenna_height_m: 2.05
```

## warmup-thermal

```yaml result
step: warmup-thermal
status: done
captures:
  warmup_min: 22
  ambient_c: 12
```

## warmup-stability

```yaml result
step: warmup-stability
status: done
captures:
  drift_nt_per_min: 0.05
  stability_min: 8
```

## log-config

```yaml result
step: log-config
status: done
captures:
  log_path: logs/2026-09-24-renfrew-walk01/mag_raw.csv
  sample_rate_hz: 1
  units: nT
  timestamp_utc: true
```

## log-write-test

```yaml result
step: log-write-test
status: done
captures:
  test_file_bytes: 2048
```

## log-disk-space

```yaml result
step: log-disk-space
status: done
captures:
  free_disk_gb: 26.5
```

## testline-select

```yaml result
step: testline-select
status: done
captures:
  test_line_id: L1
```

## testline-before

```yaml result
step: testline-before
status: done
captures:
  before_mean_nt: 52641.3
  before_sigma_nt: 4.8
```

## testline-after

```yaml result
step: testline-after
status: skipped
reason: Session ended early; repeat pass not walked
```

This is the check that would have bounded the drift across the session. Its absence is
the main limitation of this dataset.

## testline-note

```yaml result
step: testline-note
status: done
```

Single clear anomaly near station 40, amplitude roughly 12 nT above background, about
6 m wide. Consistent with the drain noted during reconnaissance.

## walk-pace

```yaml result
step: walk-pace
status: done
captures:
  target_pace_mps: 0.5
  observed_pace_mps: 0.55
```

## walk-height

```yaml result
step: walk-height
status: done
captures:
  sensor_height_m: 1.0
```

Sensor carried on a fixed-length staff at 1.0 m, matching the base station height.

## walk-direction

```yaml result
step: walk-direction
status: done
captures:
  line_azimuth_deg: 0
```

## walk-offline

```yaml result
step: walk-offline
status: done
```

## walk-log

```yaml result
step: walk-log
status: done
captures:
  line_id: L1, L2, L3
  start_station: 0
  end_station: 120
```

L1 to L3 complete. L4 not started.

## midcheck-reference

```yaml result
step: midcheck-reference
status: done
captures:
  reference_station_id: REF-395-A
  reference_value_nt: 52694.1
  reference_time: 2026-09-24T18:30:00Z
```

Offset from the pre-survey occupation is 0.7 nT, within the base station's own
variation over the same interval.

## midcheck-base

```yaml result
step: midcheck-base
status: done
captures:
  base_gap_count: 0
  base_still_running: true
```

## midcheck-power

```yaml result
step: midcheck-power
status: done
captures:
  battery_v: 11.7
  free_disk_gb: 26.4
```

Projected to roughly 10 hours of remaining runtime, above the session requirement.

## midcheck-gnss

```yaml result
step: midcheck-gnss
status: done
captures:
  fix_type: rtk-fixed
```

## packdown-stop

```yaml result
step: packdown-stop
status: done
```

## packdown-backup

```yaml result
step: packdown-backup
status: done
captures:
  backup_primary: workstation /data/field-sop/2026-09-24-renfrew-walk01
  backup_secondary: USB drive FIELDDATA-01
```

## packdown-attach

```yaml result
step: packdown-attach
status: done
captures:
  logs_attached: 2
  hash_mismatch: false
```

## packdown-closeout

```yaml result
step: packdown-closeout
status: done
captures:
  run_status: partial
```

## walk-session-closeout

```yaml result
step: walk-session-closeout
status: done
```

Three of nine lines completed. L4 to L9 not walked. The test line was not repeated, so
intra-session drift is unbounded for this dataset; treat it as reconnaissance only.

One observation moved to `runs/_inbox/`: the staff showed a visible oscillation at the
wind speed present, which is worth writing up as a limiting condition.
