<script lang="ts">
  // A structured editor for the `devices` setting: other equipment (a GNSS receiver, a
  // base station, a UAV, a battery, a ground station, ...), each with a kind, a name, and
  // its serial numbers. It mirrors `SensorsEditor`, but the stored form is
  // `kind/name: serial, serial; name`; the parser and formatter are shared with Rust
  // through `crates/sop-core/tests/devices-vectors.json`.
  import { formatDevices, parseDevices, deviceSeparatorIn, type DeviceStock } from "../lib/devices";

  let { value, onChange }: { value: string; onChange: (next: string) => void } = $props();

  // Common kinds, offered as suggestions. The operator can type any label.
  const KINDS = [
    "GNSS receiver",
    "Base station",
    "UAV",
    "Battery",
    "Laptop/GCS",
    "Telemetry radio",
    "Mount/Tripod",
    "Other",
  ];

  let devices: DeviceStock[] = $state([]);
  let error = $state("");

  // Re-sync only when the parent value really changed (a different setting or a reload),
  // compared in formatted form so a blank row we are holding does not look like a change.
  $effect(() => {
    const incoming = parseDevices(value);
    if (formatDevices(incoming) !== formatDevices(devices)) devices = incoming;
  });

  const firstProblem = (candidate: DeviceStock[]): string => {
    for (const device of candidate) {
      const fields: Array<[string, string]> = [
        ["kind", device.kind],
        ["name", device.name],
        ...device.serials.map((serial): [string, string] => ["serial", serial]),
      ];
      for (const [what, text] of fields) {
        const separator = deviceSeparatorIn(text);
        if (separator) return `A device ${what} cannot contain '${separator}'.`;
      }
    }
    return "";
  };

  // Keep the raw text locally (so a half-typed name is visible), but only hand a valid
  // value back to the parent; an illegal character shows an error instead of being stored.
  const emit = (next: DeviceStock[]): void => {
    devices = next;
    const problem = firstProblem(next);
    error = problem;
    if (!problem) onChange(formatDevices(next));
  };

  const addDevice = (): void => emit([...devices, { kind: "", name: "", serials: [] }]);
  const removeDevice = (index: number): void => emit(devices.filter((_, i) => i !== index));
  const patch = (index: number, changes: Partial<DeviceStock>): void =>
    emit(devices.map((device, i) => (i === index ? { ...device, ...changes } : device)));
  const addSerial = (index: number): void => {
    const device = devices[index];
    patch(index, { serials: [...(device?.serials ?? []), ""] });
  };
  const setSerial = (index: number, serialIndex: number, serial: string): void => {
    const serials = (devices[index]?.serials ?? []).map((known, j) => (j === serialIndex ? serial : known));
    patch(index, { serials });
  };
  const removeSerial = (index: number, serialIndex: number): void => {
    const serials = (devices[index]?.serials ?? []).filter((_, j) => j !== serialIndex);
    patch(index, { serials });
  };

  // Group the rows by kind for display, keeping the first-seen order of the kinds and of
  // the devices within each kind.
  const groups: Array<{ kind: string; items: Array<{ device: DeviceStock; index: number }> }> =
    $derived.by(() => {
      const out: Array<{ kind: string; items: Array<{ device: DeviceStock; index: number }> }> = [];
      devices.forEach((device, index) => {
        const label = device.kind.trim();
        let group = out.find((candidate) => candidate.kind === label);
        if (!group) {
          group = { kind: label, items: [] };
          out.push(group);
        }
        group.items.push({ device, index });
      });
      return out;
    });
</script>

<datalist id="device-kinds">
  {#each KINDS as kind (kind)}
    <option value={kind}></option>
  {/each}
</datalist>

<div class="devices">
  {#each groups as group (group.kind)}
    <div class="group">
      <div class="group-head">{group.kind || "(no kind)"}</div>
      {#each group.items as item (item.index)}
        <div class="device">
          <div class="row">
            <input
              class="kind"
              list="device-kinds"
              placeholder="kind, e.g. GNSS receiver"
              value={item.device.kind}
              oninput={(e) => patch(item.index, { kind: (e.currentTarget as HTMLInputElement).value })}
            />
            <input
              class="name"
              placeholder="name, e.g. Trimble R10"
              value={item.device.name}
              oninput={(e) => patch(item.index, { name: (e.currentTarget as HTMLInputElement).value })}
            />
            <button type="button" onclick={() => removeDevice(item.index)} title="Remove this device">
              Remove
            </button>
          </div>
          <div class="serials">
            {#each item.device.serials as serial, j (j)}
              <span class="serial">
                <input
                  placeholder="serial"
                  value={serial}
                  oninput={(e) => setSerial(item.index, j, (e.currentTarget as HTMLInputElement).value)}
                />
                <button type="button" onclick={() => removeSerial(item.index, j)} title="Remove this serial">
                  &times;
                </button>
              </span>
            {/each}
            <button type="button" class="add" onclick={() => addSerial(item.index)}>+ serial</button>
          </div>
        </div>
      {/each}
    </div>
  {/each}
  <button type="button" onclick={addDevice}>+ device</button>
  {#if error}
    <p class="err">{error}</p>
  {/if}
</div>

<style>
  .devices { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; width: 100%; }
  .group { border: 1px solid var(--line); border-radius: 6px; padding: 8px; width: 100%; }
  .group-head { font-size: 12px; text-transform: uppercase; letter-spacing: .06em; color: var(--muted); margin-bottom: 6px; }
  .device { padding: 4px 0; }
  .row { display: flex; gap: 8px; align-items: center; }
  .kind { max-width: 20ch; }
  .name { max-width: 24ch; }
  .serials { display: flex; flex-wrap: wrap; gap: 4px 8px; align-items: center; margin-top: 6px; }
  .serial { display: inline-flex; align-items: center; gap: 2px; }
  .serial input { width: 10ch; }
  .serial button { padding: 0 6px; }
  .add { font-size: 12px; }
</style>
