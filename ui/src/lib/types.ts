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
  path: string;
  step_count: number;
  unresolved_includes: string[];
  steps: StepEntry[];
}

export interface RunEntry {
  run_id: unknown;
  sop: unknown;
  sop_version: unknown;
  operator: unknown;
  site: unknown;
  started: unknown;
  status: unknown;
  deviations_count: unknown;
  path: string;
  step_count: number;
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

export interface StepPatch {
  title?: string | null;
  kind?: string | null;
  severity?: string | null;
  deprecated?: boolean | null;
  prose?: string | null;
}