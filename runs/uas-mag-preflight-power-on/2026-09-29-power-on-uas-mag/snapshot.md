---
kind: snapshot
---

## Supply

```yaml step
id: supply
kind: check
```

- [ ] Supply within the sensor's operating range
- [ ] No brown-out during boot
- [ ] Record the supply voltage
- [ ] Note the power source (flight battery / bench supply / external DC)

## Firmware

```yaml step
id: firmware
kind: check
```

- [ ] Firmware version logged for the record
- [ ] No error codes on boot

## Self-test

```yaml step
id: self-test
kind: check
```

- [ ] Sensor self-test completed
- [ ] Data stream present at the configured rate
- [ ] Self-test marked as passed

