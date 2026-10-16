import { expect, test } from "./fixtures";

test("a test recording can be made and played back", async ({ app }) => {
  await app.getByRole("button", { name: "Aufnehmen" }).click();
  await expect(app.getByRole("timer")).toContainText("0:01 / 0:30");
  await app.getByRole("button", { name: "Stopp" }).click();
  await expect(app.getByText("Eher leise – „Pegel einmessen“ oben hilft.")).toBeVisible();
  await app.getByRole("button", { name: "Abspielen" }).click();
  await expect(app.getByRole("button", { name: "Wiedergabe stoppen" })).toBeVisible();
  // The playback ends by itself after the recording's length
  await expect(app.getByRole("button", { name: "Abspielen" })).toBeVisible({ timeout: 5000 });
});
