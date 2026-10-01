// The `devices` setting, parsed on the renderer side.
//
// This mirrors `sop_core::settings::parse_devices` / `format_devices` exactly; the same
// `crates/sop-core/tests/devices-vectors.json` drives both (see `npm run test:devices`),
// so the two cannot drift. The renderer only edits the setting; the shell still owns it.

export interface DeviceStock {
  kind: string;
  name: string;
  serials: string[];
}

// The characters that mean something in the `devices` line, so a name or serial that
// contains one is rejected rather than silently re-parsed into a different device.
const DEVICE_SEPARATORS = [";", ":", ",", "/"];

function deviceSeparatorIn(value: string): string | null {
  for (const character of value) {
    if (DEVICE_SEPARATORS.includes(character)) return character;
  }
  return null;
}

function pushSerials(serials: string[], rest: string): void {
  for (const raw of rest.split(",")) {
    const serial = raw.trim();
    if (serial && !serials.includes(serial)) serials.push(serial);
  }
}

// Parse `kind/name: serial, serial; kind/name; name`. The `kind/` prefix is optional and
// a device written twice is merged; two devices merge only when kind and name both match.
export function parseDevices(text: string): DeviceStock[] {
  const out: DeviceStock[] = [];
  for (const entry of text.split(";")) {
    const trimmed = entry.trim();
    if (!trimmed) continue;
    const colon = trimmed.indexOf(":");
    const head = (colon === -1 ? trimmed : trimmed.slice(0, colon)).trim();
    const rest = colon === -1 ? "" : trimmed.slice(colon + 1);
    if (!head) continue;
    const slash = head.indexOf("/");
    const kind = slash === -1 ? "" : head.slice(0, slash).trim();
    const name = (slash === -1 ? head : head.slice(slash + 1)).trim();
    if (!name) continue;
    const existing = out.find((device) => device.kind === kind && device.name === name);
    if (existing) {
      pushSerials(existing.serials, rest);
    } else {
      const serials: string[] = [];
      pushSerials(serials, rest);
      out.push({ kind, name, serials });
    }
  }
  return out;
}

// Render the setting so it round-trips through `parseDevices`.
export function formatDevices(devices: DeviceStock[]): string {
  return devices
    .filter((device) => device.name.trim() !== "")
    .map((device) => {
      const kind = device.kind.trim();
      const name = device.name.trim();
      const label = kind === "" ? name : `${kind}/${name}`;
      return device.serials.length === 0 ? label : `${label}: ${device.serials.join(", ")}`;
    })
    .join("; ");
}

export { deviceSeparatorIn };
