// Drafts are local to this machine and this explicitly selected data directory.
export function readDraft(key: string): string {
  try { return localStorage.getItem(`field-sop:draft:${key}`) ?? ""; }
  catch { return ""; }
}

export function writeDraft(key: string, value: string): void {
  try {
    if (value) localStorage.setItem(`field-sop:draft:${key}`, value);
    else localStorage.removeItem(`field-sop:draft:${key}`);
  } catch { /* Recording still works when browser storage is unavailable. */ }
}
