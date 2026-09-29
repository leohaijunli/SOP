// The `sensors` setting: sensor models and the serial numbers of the units the machine
// has used. The storage form is `model: serial, serial; model`, matching
// `sop_core::settings`. The editor and the start-a-run picker share these helpers so the
// two cannot drift.

export type SensorStock = { model: string; serials: string[] };

export const parseSensors = (value: string): SensorStock[] => {
  const out: SensorStock[] = [];
  for (const entry of value.split(";")) {
    const trimmed = entry.trim();
    if (!trimmed) continue;
    const colon = trimmed.indexOf(":");
    const model = (colon < 0 ? trimmed : trimmed.slice(0, colon)).trim();
    if (!model) continue;
    const serials = (colon < 0 ? "" : trimmed.slice(colon + 1))
      .split(",")
      .map((serial) => serial.trim())
      .filter((serial) => serial !== "");
    const known = out.find((stock) => stock.model === model);
    if (known) {
      for (const serial of serials) if (!known.serials.includes(serial)) known.serials.push(serial);
    } else {
      out.push({ model, serials });
    }
  }
  return out;
};

export const formatSensors = (stocks: SensorStock[]): string =>
  stocks
    .filter((stock) => stock.model.trim() !== "" || stock.serials.some((s) => s.trim() !== ""))
    .map((stock) => {
      const model = stock.model.trim() || "model";
      const serials = stock.serials.map((serial) => serial.trim()).filter((serial) => serial !== "");
      return serials.length ? `${model}: ${serials.join(", ")}` : model;
    })
    .join("; ");
