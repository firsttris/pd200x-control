import { currentLang } from "./i18n";

const formatters = new Map<string, Intl.NumberFormat>();

function formatter(digits: number, signed: boolean): Intl.NumberFormat {
  const key = `${currentLang()}-${digits}-${signed}`;
  let f = formatters.get(key);
  if (!f) {
    f = new Intl.NumberFormat(currentLang(), {
      minimumFractionDigits: digits,
      maximumFractionDigits: digits,
      signDisplay: signed ? "exceptZero" : "auto",
      // "1.000" would read as 1 when typed back into a field.
      useGrouping: false,
    });
    formatters.set(key, f);
  }
  return f;
}

/** `−6,4 dB`, `+14,6 dB` (German) or `-6.4 dB` (English). */
export function formatDb(value: number, { digits = 1, signed = false } = {}): string {
  return `${formatter(digits, signed).format(value).replace("-", "−")} dB`;
}

export function formatNumber(value: number, digits = 0): string {
  return formatter(digits, false).format(value).replace("-", "−");
}

/** `80 Hz`, `1,3 kHz`, `22 kHz`. */
export function formatHz(hz: number): string {
  if (hz < 1000) return `${Math.round(hz)} Hz`;
  const k = hz / 1000;
  return `${formatNumber(k, k < 10 && !Number.isInteger(k) ? 1 : 0)} kHz`;
}

/** PipeWire and the desktops show the volume in percent; it is cubic: 175 % is +14,6 dB. */
export function percentToDb(percent: number): number {
  return Math.round(60 * Math.log10(percent / 100) * 10) / 10;
}

export function dbToPercent(db: number): number {
  return Math.round(10 ** (db / 60) * 100);
}
