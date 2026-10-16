/**
 * The UI language. Texts live in messages/{de,en}.json and are compiled by
 * Paraglide to src/paraglide: components call them directly (`m.nav_basics()`).
 * The backend sends error kinds, which `errorText()` turns into text.
 *
 * The language comes from the setting (localStorage), else from the system:
 * German for de-*, English for everything else.
 */
import { m } from "@/paraglide/messages";
import { overwriteGetLocale } from "@/paraglide/runtime";

export type Lang = "de" | "en";
export const LANGS: { id: Lang; label: string }[] = [
  { id: "de", label: "Deutsch" },
  { id: "en", label: "English" },
];

const STORAGE_KEY = "pd200x_lang";

const isLang = (v: unknown): v is Lang => v === "de" || v === "en";

export function storedLang(): Lang | null {
  try {
    const v = localStorage.getItem(STORAGE_KEY);
    return isLang(v) ? v : null;
  } catch {
    return null;
  }
}

export function systemLang(languages: readonly string[] = navigator.languages ?? [navigator.language]): Lang {
  const first = languages.find(Boolean) ?? "en";
  return first.toLowerCase().startsWith("de") ? "de" : "en";
}

let current: Lang | undefined;

export function currentLang(): Lang {
  current ??= storedLang() ?? systemLang();
  return current;
}

overwriteGetLocale(() => currentLang());

/** Switches the language and reloads; `null` follows the system again. */
export function setLang(lang: Lang | null) {
  try {
    if (lang) localStorage.setItem(STORAGE_KEY, lang);
    else localStorage.removeItem(STORAGE_KEY);
  } catch {
    // Without storage the choice lasts until the next start.
  }
  current = lang ?? systemLang();
  window.location.reload();
}

/** Text for an error kind from the backend. */
export function errorText(kind: string): string {
  switch (kind) {
    case "notFound":
      return m.error_notFound();
    case "permissionDenied":
      return m.error_permissionDenied();
    case "disconnected":
      return m.error_disconnected();
    case "timeout":
      return m.error_timeout();
    case "noAudioSource":
      return m.error_noAudioSource();
    case "toolMissing":
      return m.error_toolMissing();
    case "noVoice":
      return m.error_noVoice();
    case "invalidValue":
      return m.error_invalidValue();
    default:
      return m.error_other();
  }
}
