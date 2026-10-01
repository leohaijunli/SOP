// Theme state. Follows the system preference by default; the operator can pin
// light or dark from the header, and the choice is remembered across launches.

export type Theme = "auto" | "light" | "dark";

const KEY = "field-sop:theme";

const apply = (t: Theme): void => {
  const el = document.documentElement;
  if (t === "dark") el.setAttribute("data-theme", "dark");
  else if (t === "light") el.setAttribute("data-theme", "light");
  else el.removeAttribute("data-theme");
};

export const loadTheme = (): Theme => {
  const stored = localStorage.getItem(KEY);
  const t: Theme = stored === "light" || stored === "dark" ? stored : "auto";
  apply(t);
  return t;
};

export const cycleTheme = (current: Theme): Theme => {
  const next: Theme =
    current === "auto" ? "light" : current === "light" ? "dark" : "auto";
  localStorage.setItem(KEY, next);
  apply(next);
  return next;
};