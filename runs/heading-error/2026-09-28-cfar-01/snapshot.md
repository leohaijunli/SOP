---
kind: snapshot
---

## Mount

```yaml step
id: mount
kind: check
```

- [ ] Sensor centered on the rotation table
- [ ] Table marked at known azimuths
- [ ] No nearby ferrous objects

## Rotate

```yaml step
id: rotate
kind: check
```

- [ ] Full 360-degree sweep recorded
- [ ] Azimuth and reading logged at each mark
- [ ] Peak-to-peak heading error computed

## Result

```yaml step
id: result
kind: check
```

- [ ] Heading error logged
- [ ] Residual error acknowledged by the operator

