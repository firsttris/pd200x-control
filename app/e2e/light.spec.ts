import { expect, mockState, test } from "./fixtures";

test("colour and brightness", async ({ app }) => {
  await app.getByRole("link", { name: "Licht" }).click();
  await app.getByRole("button", { name: "Blau" }).click();
  await expect(app.getByText("Vorschau · Blau · 70 %")).toBeVisible();
  const slider = app.getByRole("slider", { name: "Helligkeit" });
  await slider.focus();
  await app.keyboard.press("Home");
  await expect(app.getByText("Vorschau · Blau · 0 %")).toBeVisible();
  await expect.poll(() => mockState<number>(app, "snapshot.settings.lighting.brightness")).toBe(0);
});
