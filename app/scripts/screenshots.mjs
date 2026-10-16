// Takes the pictures in docs/ (README, documentation, banner, social preview)
// from the demo data of the mock backend. scripts/screenshots.sh serves the
// build; by hand: node scripts/screenshots.mjs <docs dir> [base url]
// The pictures show the English UI (the browser's language decides). The clock
// is fixed, so an unchanged page gives an unchanged picture.
import { readFileSync } from "node:fs";
import { chromium } from "@playwright/test";

const OUT = process.argv[2] ?? "../docs";
const BASE = process.argv[3] ?? "http://127.0.0.1:4317";
const NOW = new Date("2026-06-15T09:42:00+02:00");

const browser = await chromium.launch({ executablePath: process.env.PW_CHROMIUM_PATH || undefined });

async function newPage(options = {}) {
  const ctx = await browser.newContext({
    viewport: { width: 1280, height: 800 },
    deviceScaleFactor: 1.5,
    locale: "en-US",
    timezoneId: "Europe/Berlin",
    colorScheme: "dark",
    ...options,
  });
  const page = await ctx.newPage();
  await page.clock.setFixedTime(NOW);
  return page;
}

const shot = async (page, name, opts = {}) => {
  // Let fonts, transitions and the next snapshot poll settle.
  await page.waitForTimeout(opts.wait ?? 1300);
  const target = opts.locator ?? page;
  await target.screenshot({ path: `${OUT}/screenshot-${name}.png` });
  console.log("shot", name);
};

/** Demo state: the microphone at full gain, the system volume at 175 %, speech in the target zone. */
const demo = (page) =>
  page.evaluate(() => {
    const s = window.__pd200xMock.state;
    s.voiceDb = -23.5;
    s.snapshot.boostDb = 14.6;
    s.snapshot.settings.boostDb = 14.6;
  });

const page = await newPage({ viewport: { width: 1280, height: 860 } });
let loads = 0;
const open = async (path) => {
  // A fresh load also gives a fresh mock backend with the demo data.
  await page.goto(`${BASE}/?load=${++loads}#${path}`);
  await page.getByRole("navigation", { name: "Sections" }).waitFor();
  await demo(page);
};
const mock = (fn) => page.evaluate(fn);

// Basics with a finished test recording that is playing.
await open("/");
// The clock stands still: move the start back once the recording or playback runs.
await page.getByRole("button", { name: "Record" }).click();
await page.getByRole("button", { name: "Stop", exact: true }).waitFor();
await mock(() => (window.__pd200xMock.state.recording.startedAt -= 12_000));
await page.getByRole("button", { name: "Stop", exact: true }).click();
await page.getByRole("button", { name: "Play" }).click();
await page.getByRole("button", { name: "Stop playback" }).waitFor();
await mock(() => (window.__pd200xMock.state.recording.startedAt -= 5_000));
await shot(page, "basics");
await shot(page, "recording", { locator: page.locator("section").filter({ hasText: "Test recording" }), wait: 300 });

// "Find my level" with its proposal.
await open("/");
await page.getByRole("button", { name: "Find my level" }).click();
await page.getByRole("button", { name: "Start" }).click();
await page.getByRole("button", { name: "Apply" }).waitFor();
await shot(page, "calibrate", { locator: page.getByRole("dialog"), wait: 400 });

// Sound: a preset with its curve, band 3 selected.
await page.setViewportSize({ width: 1280, height: 860 });
await open("/sound");
await page.getByRole("button", { name: "Natural", exact: true }).click();
await page.getByRole("tab", { name: /^3/ }).click();
await page.evaluate(() => document.activeElement?.blur());
await shot(page, "sound");

// Light: the ring in cyan.
await page.setViewportSize({ width: 1280, height: 800 });
await open("/light");
await shot(page, "light");

// Light theme, for the documentation.
const light = await newPage({ colorScheme: "light", viewport: { width: 1280, height: 860 } });
await light.goto(`${BASE}/?load=light#/sound`);
await light.getByRole("navigation", { name: "Sections" }).waitFor();
await light.getByRole("button", { name: "Bright", exact: true }).click();
await light.evaluate(() => document.activeElement?.blur());
await shot(light, "sound-light");

// Banner and social preview: the basics page under the name.
const art = await newPage({ viewport: { width: 1280, height: 640 }, deviceScaleFactor: 1 });
const basics = readFileSync(`${OUT}/screenshot-basics.png`).toString("base64");
const icon = readFileSync(new URL("../src-tauri/icons/128x128@2x.png", import.meta.url)).toString("base64");
// The app's font, so the banner looks like the app also where Geist is not installed.
const geist = readFileSync(
  new URL("../node_modules/@fontsource-variable/geist/files/geist-latin-wght-normal.woff2", import.meta.url),
).toString("base64");
const card = (width, height, social) => `<!doctype html><html><head><style>@font-face{font-family:Geist;
  font-weight:100 900;src:url(data:font/woff2;base64,${geist}) format("woff2")}</style></head><body style="margin:0;width:${width}px;height:${height}px;
  background:radial-gradient(circle at 20% 10%,#1f2733,#0e0f11 60%);font-family:Geist,Inter,system-ui,sans-serif;color:#e8eaed;
  display:flex;align-items:center;justify-content:center;gap:56px;padding:0 64px;box-sizing:border-box;overflow:hidden">
  <div style="flex:0 0 auto;max-width:${social ? 500 : 640}px">
    <div style="display:flex;align-items:center;gap:18px">
      <img src="data:image/png;base64,${icon}" width="72" height="72">
      <span style="font-size:52px;font-weight:700;letter-spacing:-1px">pd200x-control</span>
    </div>
    <p style="font-size:28px;line-height:1.35;color:#cfd3da;margin:26px 0 0">Your Maono PD200X, finally on Linux.</p>
    <p style="font-size:18px;line-height:1.55;color:#8f949c;margin:16px 0 0">Gain, monitoring, noise cancellation, EQ, compressor,
      limiter and the RGB ring · live level · test recording · a level finder for quiet dynamic mics</p>
  </div>
  <img src="data:image/png;base64,${basics}" style="height:${Math.round(height * 0.86)}px;border-radius:14px;border:1px solid #33363d;box-shadow:0 20px 60px rgba(0,0,0,.6)">
</body></html>`;
await art.setContent(card(1280, 640, true));
await art.waitForTimeout(300);
await art.screenshot({ path: `${OUT}/social-preview.png` });
console.log("shot social-preview");
await art.setViewportSize({ width: 1800, height: 560 });
await art.setContent(card(1800, 560, false));
await art.waitForTimeout(300);
await art.screenshot({ path: `${OUT}/banner.png` });
console.log("shot banner");

await browser.close();
