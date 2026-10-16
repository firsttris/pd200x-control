import { expect, mockState, test } from "./fixtures";

test("a preset, then dragging an EQ point makes it custom", async ({ app }) => {
  await app.getByRole("link", { name: "Klang" }).click();
  await app.getByRole("button", { name: "Natürlich" }).click();
  await expect(app.getByRole("button", { name: "Natürlich", pressed: true })).toBeVisible();

  const handle = app.getByRole("slider", { name: "Band 3" });
  const box = (await handle.boundingBox())!;
  await app.mouse.move(box.x + box.width / 2, box.y + box.height / 2);
  await app.mouse.down();
  await app.mouse.move(box.x + box.width / 2 + 60, box.y - 30, { steps: 6 });
  await app.mouse.up();

  await expect(app.getByText("Eigene Einstellung")).toBeVisible();
  const band = await mockState<{ frequency: number; gainDb: number }>(app, "snapshot.settings.tone.eq.2");
  expect(band.frequency).toBeGreaterThan(1320);
  expect(band.gainDb).toBeGreaterThan(2.5);
});

test("an own profile can be saved and picked again", async ({ app }) => {
  await app.getByRole("link", { name: "Klang" }).click();
  await app.getByRole("button", { name: "Tief", exact: true }).click();
  await app.getByRole("button", { name: "Eigenes Profil" }).click();
  await app.getByLabel("Name").fill("Abendstimme");
  await app.getByRole("button", { name: "Speichern" }).click();
  await expect(app.getByRole("button", { name: "Abendstimme", exact: true, pressed: true })).toBeVisible();
  await app.getByRole("button", { name: "Neutral", exact: true }).click();
  await app.getByRole("button", { name: "Abendstimme", exact: true }).click();
  await expect.poll(() => mockState<string>(app, "snapshot.settings.tone.source")).toBe("abendstimme");
});
