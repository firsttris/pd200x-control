import { expect, mockState, test } from "./fixtures";

test("the level meter moves and mute stops it", async ({ app }) => {
  const meter = app.getByRole("meter");
  await expect(meter).toBeVisible();
  await expect.poll(async () => Number(await meter.getAttribute("aria-valuenow"))).toBeGreaterThan(-50);
  await app.getByRole("button", { name: "Stummschalten" }).click();
  await expect(app.getByRole("button", { name: "Stummgeschaltet" })).toBeVisible();
  await expect.poll(async () => Number(await meter.getAttribute("aria-valuenow"))).toBe(-60);
});

test("calibration proposes and applies gain and boost", async ({ app }) => {
  await app.getByRole("button", { name: "Pegel einmessen" }).click();
  await app.getByRole("button", { name: "Los geht's" }).click();
  await expect(app.getByText(/Sprich jetzt/)).toBeVisible();
  const dialog = app.getByRole("dialog");
  await expect(dialog.getByText("+12,5 dB")).toBeVisible({ timeout: 10_000 });
  await dialog.getByRole("button", { name: "Übernehmen" }).click();
  await expect(app.getByText("162 %")).toBeVisible();
  expect(await mockState<number>(app, "snapshot.boostDb")).toBe(12.5);
});

test("dragging the gain slider sends the new value", async ({ app }) => {
  const thumb = app.getByRole("slider", { name: "Mikrofon-Gain" });
  const box = (await thumb.boundingBox())!;
  const y = box.y + box.height / 2;
  await app.mouse.move(box.x + box.width / 2, y);
  await app.mouse.down();
  // Half the track to the left (the slider spans most of the card)
  const width = app.viewportSize()!.width;
  await app.mouse.move(box.x - width * 0.3, y, { steps: 8 });
  await app.mouse.up();
  await expect.poll(() => mockState<number>(app, "snapshot.live.gain")).toBeLessThan(60);
});
