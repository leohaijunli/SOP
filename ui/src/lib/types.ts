// The shapes the Rust shell hands to the renderer, and the shapes the renderer sends
// back. The frontend holds no rules; these are the wire types from `sop-app/src/api.rs`,
// mirrored as closely as TypeScript allows.

export interface Status {
  workingCopy: string;
  settingsPath: string;
  projectTitle: string | null;
  projectId: string | null;
  institution: string | null;
  git: GitInfo;
  content: ContentCounts;
  validation: ValidationCounts;
}

export interface ValidationCounts {
  errors: number;
  warnings: number;
}

export interface GitInfo {
  isRepository: boolean;
  branch: string | null;
  remote: string;
  url: string | null;
  urlHasCredentials: boolean;
  dirty: boolean;
  ahead: number | null;
  behind: number | null;
}

export interface ContentCounts {
  procedures: number;
  checklists: number;
  runs: number;
  help: number;
  logDirectories: number;
}

export interface SettingRow {
  key: string;
  value: string | null;
  description: string;
  /// Kept in the working copy rather than on this machine.
  inRepository: boolean;
}

export interface ProjectField {
  key: string;
  value: string | null;
  description: string;
}

export interface ProjectView {
  path: string;
  fields: ProjectField[];
}

export interface ProcedureChoice {
  path: string;
  id: string | null;
  title: string | null;
  included: boolean;
}

// ---- the manifest (dist/manifest.json) --------------------------------------

export interface Manifest {
  schema: number;
  project: ProjectEntry | null;
  procedures: ProcedureEntry[];
  checklists: ChecklistEntry[];
  testplans: TestPlan[];
  runs: RunEntry[];
  help: HelpEntry[];
  help_sections: HelpSection[];
}

export interface ProjectEntry {
  project_id: unknown;
  title: unknown;
  institution: unknown;
  lead: unknown;
  started: unknown;
  updated: unknown;
  summary: unknown;
  applies_to: unknown[];
  tags: unknown[];
  sites: unknown[];
  path: string;
  body: string;
}

export interface HelpEntry {
  help_id: unknown;
  title: unknown;
  section: unknown;
  order: unknown;
  audience: unknown;
  summary: unknown;
  updated: unknown;
  tags: unknown[];
  path: string;
  body: string;
}

export interface HelpSection {
  title: string;
  order: number;
}

export interface ProcedureEntry {
  procedure_id: unknown;
  title: unknown;
  version: unknown;
  updated: unknown;
  applies_to: unknown[];
  tags: unknown[];
  path: string;
  steps: StepEntry[];
}

export interface ChecklistEntry {
  sop_id: unknown;
  title: unknown;
  version: unknown;
  updated: unknown;
  status: unknown;
  applies_to: unknown[];
  equipment: unknown[];
  conditions: ConditionDecl[];
  path: string;
  step_count: number;
  unresolved_includes: string[];
  steps: StepEntry[];
}

export interface TestCase {
  id: string;
  /// The checklist id a run of this case records.
  sop_id: string;
  title: unknown;
  path: string;
  step_count: number;
  order: number;
}

export interface TestPlan {
  id: string;
  title: unknown;
  path: string;
  order: number;
  cases: TestCase[];
}

export interface RunEntry {
  run_id: unknown;
  sop: unknown;
  sop_version: unknown;
  operator: unknown;
  site: unknown;
  plan: unknown;
  case: unknown;
  started: unknown;
  status: unknown;
  deviations_count: unknown;
  sensor: { model?: unknown; serial?: unknown; firmware?: unknown } | null;
  conditions: Record<string, unknown> | null;
  path: string;
  step_count: number;
}

/// A value the checklist asks the operator to record at run start.
export interface ConditionDecl {
  key: string;
  hint?: string | null;
}

export interface StepEntry {
  id: unknown;
  title: unknown;
  kind: unknown;
  severity: unknown;
  deprecated: boolean;
  captures: Capture[];
  body: string;
  source: unknown;
}

export interface Capture {
  key: string;
  label?: unknown;
  type?: unknown;
  unit?: unknown;
  required?: boolean;
  options?: unknown[];
  expected?: unknown;
  [key: string]: unknown;
}

// ---- edits the renderer sends back ------------------------------------------

export interface CaptureInput {
  key: string;
  label: string;
  type: string;
  unit: string | null;
  required: boolean;
  options: string[];
  expected: ExpectedInput | null;
}

export type ExpectedInput =
  | { form: "range"; min: number | null; max: number | null }
  | { form: "bool"; value: boolean }
  | { form: "select"; value: string };

export interface StepInput {
  id: string;
  title: string;
  kind: string;
  severity: string;
  deprecated: boolean;
  prose: string;
  captures: CaptureInput[];
}

// ---- a running (or ended) run, for the execution and history views -------------------

export interface RunView {
  sop: string;
  runId: string;
  operator: string | null;
  site: string | null;
  plan: string | null;
  case: string | null;
  started: string | null;
  ended: string | null;
  runStatus: string | null;
  snapshotSha256: string | null;
  sopVersion: string | null;
  sopCommit: string | null;
  deviationsCount: number;
  sensor: SensorView | null;
  hardware: string[];
  conditions: Record<string, string>;
  steps: RunStepView[];
  runNotes: string[];
  runAttachments: RunAttachmentView[];
  recordPath: string;
}

export interface SensorView {
  model: string | null;
  serial: string | null;
  firmware: string | null;
}

/// What the start panel sends with the run's identity. Every part is optional.
export interface RunMetaInput {
  sensorModel: string | null;
  sensorSerial: string | null;
  sensorFirmware: string | null;
  hardware: string[];
  conditions: Record<string, string>;
  plan?: string | null;
  case?: string | null;
}

export interface RunStepView {
  id: string;
  title: string;
  prose: string;
  severity: string | null;
  kind: string | null;
  status: string;
  reason: string | null;
  checklist: ChecklistItemView[];
  captures: RunCaptureView[];
  notes: string[];
  attachments: RunAttachmentView[];
}

export interface ChecklistItemView {
  text: string;
  checked: boolean;
}

export interface RunCaptureView {
  key: string;
  label: string | null;
  type: string | null;
  unit: string | null;
  required: boolean;
  options: string[];
  expected: unknown;
  value: string | null;
  acknowledged: boolean;
}

export interface RunAttachmentView {
  path: string;
  sha256: string;
  size: number;
}

// A run event as the Rust shell deserializes it: variant tag is PascalCase, fields
// are snake_case (the Rust names).
export type RunEventInput =
  | { type: "StepOpened"; at: string; step: string }
  | { type: "CheckboxToggled"; at: string; step: string; index: number; checked: boolean }
  | { type: "CaptureRecorded"; at: string; step: string; key: string; value: string; unit: string | null }
  | { type: "CaptureCleared"; at: string; step: string; key: string; reason: string }
  | { type: "CaptureAcknowledged"; at: string; step: string; key: string; reason: string | null }
  | { type: "NoteAdded"; at: string; step: string | null; text: string }
  | {
      type: "AttachmentAdded";
      at: string;
      step: string | null;
      path: string;
      sha256: string;
      size: number;
    }
  | { type: "StepStatusChanged"; at: string; step: string; status: string; reason: string | null }
  | { type: "RunEnded"; at: string; status: string };

/// Whether the checklist a run was started from has changed since its snapshot.
export interface Drift {
  startedVersion: string | null;
  startedCommit: string | null;
  snapshotSha256: string | null;
  currentSha256: string | null;
  currentVersion: string | null;
  drifted: boolean;
}

// ---- publishing the working copy --------------------------------------------

export interface ExportResult {
  path: string;
  bytes: number;
  text: string;
}

export interface PushResult {
  branch: string | null;
  remote: string;
  changed: number;
  commit: string | null;
  upToDate: boolean;
  log: string[];
}

export interface StepPatch {
  title?: string | null;
  kind?: string | null;
  severity?: string | null;
  deprecated?: boolean | null;
  prose?: string | null;
}
