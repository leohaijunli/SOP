<script lang="ts">
  // A structured editor for the `sensors` setting: models, each with its serial numbers.
  // The stored form is a string, so every edit formats back to it and the parent writes
  // the setting the same way it writes any other. A local copy holds blank rows that an
  // "add" just created, which the formatted string would otherwise drop.
  import { parseSensors, formatSensors, type SensorStock } from "../lib/sensors";

  let { value, onChange }: { value: string; onChange: (next: string) => void } = $props();

  let stocks: SensorStock[] = $state([]);

  // Re-sync only when the parent value really changed (a different setting or a reload),
  // compared in formatted form so a blank row we are holding does not look like a change.
  $effect(() => {
    const incoming = parseSensors(value);
    if (formatSensors(incoming) !== formatSensors(stocks)) stocks = incoming;
  });

  const emit = (next: SensorStock[]): void => {
    stocks = next;
    onChange(formatSensors(next));
  };

  const addModel = (): void => emit([...stocks, { model: "", serials: [] }]);
  const removeModel = (index: number): void => emit(stocks.filter((_, i) => i !== index));
  const setModel = (index: number, model: string): void =>
    emit(stocks.map((stock, i) => (i === index ? { ...stock, model } : stock)));
  const addSerial = (index: number): void =>
    emit(stocks.map((stock, i) => (i === index ? { ...stock, serials: [...stock.serials, ""] } : stock)));
  const setSerial = (index: number, serialIndex: number, serial: string): void =>
    emit(
      stocks.map((stock, i) =>
        i === index
          ? { ...stock, serials: stock.serials.map((known, j) => (j === serialIndex ? serial : known)) }
          : stock
      )
    );
  const removeSerial = (index: number, serialIndex: number): void =>
    emit(
      stocks.map((stock, i) =>
        i === index ? { ...stock, serials: stock.serials.filter((_, j) => j !== serialIndex) } : stock
      )
    );

  // A row with no model is one the operator has not finished typing; treat its serials as
  // belonging to nothing and keep the row rather than dropping it.
  const serials = (index: number): string[] => stocks[index]?.serials ?? [];
</script>

<div class="sensors">
  {#each stocks as stock, i (i)}
    <div class="model">
      <div class="row">
        <input
          class="model-name"
          placeholder="model, e.g. UAS-MAG"
          value={stock.model}
          oninput={(e) => setModel(i, (e.currentTarget as HTMLInputElement).value)}
        />
        <button type="button" onclick={() => removeModel(i)} title="Remove this model">Remove</button>
      </div>
      <div class="serials">
        {#each serials(i) as serial, j (j)}
          <span class="serial">
            <input
              placeholder="serial"
              value={serial}
              oninput={(e) => setSerial(i, j, (e.currentTarget as HTMLInputElement).value)}
            />
            <button type="button" onclick={() => removeSerial(i, j)} title="Remove this serial">&times;</button>
          </span>
        {/each}
        <button type="button" class="add" onclick={() => addSerial(i)}>+ serial</button>
      </div>
    </div>
  {/each}
  <button type="button" onclick={addModel}>+ model</button>
</div>

<style>
  .sensors { display: flex; flex-direction: column; gap: 8px; align-items: flex-start; }
  .model { border: 1px solid var(--line); border-radius: 6px; padding: 8px; width: 100%; }
  .row { display: flex; gap: 8px; align-items: center; }
  .model-name { max-width: 28ch; }
  .serials { display: flex; flex-wrap: wrap; gap: 4px 8px; align-items: center; margin-top: 6px; }
  .serial { display: inline-flex; align-items: center; gap: 2px; }
  .serial input { width: 10ch; }
  .serial button { padding: 0 6px; }
  .add { font-size: 12px; }
</style>
