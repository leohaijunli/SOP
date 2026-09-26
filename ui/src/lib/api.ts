// The only module allowed to talk to the Rust shell. Every call is one Tauri command,
// and every command is either a query to display or an edit to hand to `sop-repo`, which
// decides whether it is allowed. Callers stay on the renderer side and never assume.

import { invoke } from "@tauri-apps/api/core";
import type {
  CaptureInput,
  Manifest,
  ProcedureChoice,
  ProjectView,
  RunEventInput,
  RunView,
  SettingRow,
  Status,
  StepInput,
  StepPatch,
} from "./types";

const text = (value: unknown): string =>
  value === null || value === undefined ? "" : String(value);

// ---- reading ----------------------------------------------------------------

export const manifest = (): Promise<Manifest> => invoke<string>("manifest_json").then(JSON.parse);

export const status = (): Promise<Status> => invoke<Status>("status");

export const validationReport = (): Promise<string> => invoke<string>("validation_report");

export const procedureChoices = (file: string | null): Promise<ProcedureChoice[]> =>
  invoke<ProcedureChoice[]>("procedure_choices", { file });

// ---- settings ---------------------------------------------------------------

export const settingsRows = (): Promise<SettingRow[]> => invoke<SettingRow[]>("settings_rows");

export const settingsPath = (): Promise<string> => invoke<string>("settings_path");

export const settingsSet = (key: string, value: string): Promise<SettingRow[]> =>
  invoke<SettingRow[]>("settings_set", { key, value });

export const settingsUnset = (key: string): Promise<SettingRow[]> =>
  invoke<SettingRow[]>("settings_unset", { key });

export const openRepository = (path: string): Promise<Status> =>
  invoke<Status>("open_repository", { repo: path });

export const remoteUrl = (): Promise<string | null> => invoke<string | null>("remote_url");

export const remoteSet = (url: string): Promise<string> => invoke<string>("remote_set", { url });

// ---- project ----------------------------------------------------------------

export const projectFields = (): Promise<ProjectView> => invoke<ProjectView>("project_fields");

export const projectSet = (key: string, value: string): Promise<ProjectView> =>
  invoke<ProjectView>("project_set", { key, value });

// ---- editing ----------------------------------------------------------------

export const stepAdd = (file: string, step: StepInput, after: string | null): Promise<string> =>
  invoke<string>("step_add", { file, step, after });

export const stepUpdate = (file: string, id: string, patch: StepPatch): Promise<string> =>
  invoke<string>("step_update", { file, id, patch });

export const stepRemove = (file: string, id: string): Promise<string> =>
  invoke<string>("step_remove", { file, id });

export const itemMove = (file: string, kind: string, id: string, up: boolean): Promise<string> =>
  invoke<string>("item_move", { file, kind, id, up });

export const captureSet = (file: string, step: string, capture: CaptureInput): Promise<string> =>
  invoke<string>("capture_set", { file, step, capture });

export const captureRemove = (file: string, step: string, key: string): Promise<string> =>
  invoke<string>("capture_remove", { file, step, key });

export const includeAdd = (file: string, target: string): Promise<string> =>
  invoke<string>("include_add", { file, target });

export const includeRemove = (file: string, target: string): Promise<string> =>
  invoke<string>("include_remove", { file, target });

// ---- runs --------------------------------------------------------------------

export const runStart = (
  sop: string,
  runId: string,
  operator: string,
  site: string,
  overrideReason: string | null
): Promise<RunView> =>
  invoke<RunView>("run_start", { sop, runId, operator, site, override: overrideReason });

export const runRecord = (sop: string, runId: string, event: RunEventInput): Promise<RunView> =>
  invoke<RunView>("run_record", { sop, runId, event });

export const runState = (sop: string, runId: string): Promise<RunView> =>
  invoke<RunView>("run_state", { sop, runId });

export const runEnd = (sop: string, runId: string, status: string): Promise<RunView> =>
  invoke<RunView>("run_end", { sop, runId, status });

export const runAttach = (sop: string, runId: string, step: string | null, path: string): Promise<RunView> =>
  invoke<RunView>("run_attach", { sop, runId, step, path });

// ---- helpers ----------------------------------------------------------------

export { text };