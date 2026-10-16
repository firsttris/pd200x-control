import { describe, expect, it } from "vitest";
import { errorText, systemLang } from "./i18n";

describe("i18n", () => {
  it("German for de-*, English otherwise", () => {
    expect(systemLang(["de-AT"])).toBe("de");
    expect(systemLang(["fr-FR", "de-DE"])).toBe("en");
    expect(systemLang([])).toBe("en");
  });

  it("every backend error kind has a text", () => {
    for (const kind of [
      "notFound",
      "permissionDenied",
      "disconnected",
      "timeout",
      "noAudioSource",
      "toolMissing",
      "noVoice",
      "invalidValue",
    ]) {
      expect(errorText(kind)).not.toBe(errorText("other"));
    }
  });
});

import de from "../../messages/de.json";
import en from "../../messages/en.json";

describe("message catalogs", () => {
  it("German and English have the same keys", () => {
    expect(Object.keys(en).sort()).toEqual(Object.keys(de).sort());
  });
});
