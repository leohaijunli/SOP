// Pure rules about captures, kept out of the component so they can be tested and so the
// component reads as layout rather than policy. The judgement itself stays with the
// operator (D4): these functions describe and check, they never decide an outcome.
import type { RunCaptureView } from "./types";

/// `number` and `integer` are the capture types whose value is a number.
export function expectsNumber(type: string | null | undefined): boolean {
  return type === "number" || type === "integer";
}

/// What the author declared the value should be, phrased for the operator to read
/// *before* typing something. Null when no expectation was declared.
export function expectedHint(capture: RunCaptureView): string | null {
  const expected = capture.expected;
  if (expected === null || expected === undefined) return null;
  const unit = capture.unit ? ` ${capture.unit}` : "";
  if (typeof expected === "object") {
    const range = expected as { min?: number; max?: number };
    if (range.min !== undefined && range.max !== undefined) return `${range.min} … ${range.max}${unit}`;
    if (range.min !== undefined) return `at least ${range.min}${unit}`;
    if (range.max !== undefined) return `at most ${range.max}${unit}`;
    return null;
  }
  if (capture.type === "select" && capture.options?.length) {
    return `one of: ${capture.options.join(", ")}`;
  }
  if (capture.type === "bool") return "true or false";
  return `exactly ${String(expected)}${unit}`;
}

/// Whether a recorded value is inside what the author declared. A value that is not a
/// number can never satisfy a numeric range: reporting `true` for `NaN` would let `12a`
/// through as acceptable.
export function expectedOk(value: string, expected: unknown): boolean {
  if (expected === null || expected === undefined) return true;
  if (typeof expected === "object" && ("min" in expected || "max" in expected)) {
    const number = Number(value);
    if (!Number.isFinite(number)) return false;
    const range = expected as { min?: number; max?: number };
    if (range.min !== undefined && number < range.min) return false;
    if (range.max !== undefined && number > range.max) return false;
    return true;
  }
  return String(expected) === value;
}

/// What is wrong with the value's *form* (not its range): an empty field is unanswered
/// rather than wrong, and a text capture has no format to be wrong about.
export function formatProblem(value: string | null | undefined, type: string | null | undefined): string | null {
  if (value === null || value === undefined || value.trim() === "") return null;
  if (!expectsNumber(type)) return null;
  const number = Number(value);
  if (!Number.isFinite(number)) return "not a number";
  if (type === "integer" && !Number.isInteger(number)) return "not a whole number";
  return null;
}

/// The labels of required captures that are still empty. A completeness gap to remind the
/// operator about, not a verdict on the step.
export function missingRequired(captures: RunCaptureView[]): string[] {
  return captures
    .filter((capture) => capture.required && (capture.value ?? "").trim() === "")
    .map((capture) => capture.label ?? capture.key);
}
