// Who is using this machine. A run records the current operator automatically, so the
// field operator signs in once instead of typing their name on every run. The choice is
// remembered across launches and can be switched from the header.

const KEY = "field-sop:operator";
const HISTORY_KEY = "field-sop:operators";

const readList = (): string[] => {
  try {
    const raw = localStorage.getItem(HISTORY_KEY);
    if (!raw) return [];
    const parsed: unknown = JSON.parse(raw);
    return Array.isArray(parsed) ? parsed.filter((x): x is string => typeof x === "string") : [];
  } catch {
    return [];
  }
};

export const getOperator = (): string => {
  return localStorage.getItem(KEY) ?? "";
};

export const getKnownOperators = (): string[] => {
  return readList();
};

/// Remember the operator and add them to the quick-pick list. Newest first.
export const setOperator = (name: string): void => {
  const trimmed = name.trim();
  if (!trimmed) return;
  localStorage.setItem(KEY, trimmed);
  const rest = readList().filter((x) => x !== trimmed);
  localStorage.setItem(HISTORY_KEY, JSON.stringify([trimmed, ...rest]));
};

export const clearOperator = (): void => {
  localStorage.removeItem(KEY);
};

/// Drop an operator from the quick-pick list. Their run records are left untouched.
/// If the removed name is the one signed in, sign out so the next launch asks again.
export const removeOperator = (name: string): void => {
  const trimmed = name.trim();
  if (!trimmed) return;
  const rest = readList().filter((x) => x !== trimmed);
  localStorage.setItem(HISTORY_KEY, JSON.stringify(rest));
  if (getOperator() === trimmed) localStorage.removeItem(KEY);
};