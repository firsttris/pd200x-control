import { expect, test } from "@playwright/test";

test.use({ locale: "en-US" });

test("the app follows an English system", async ({ page }) => {
  await page.goto("/#/");
  await expect(page.getByRole("navigation", { name: "Sections" })).toBeVisible();
  await expect(page.getByRole("button", { name: "Find my level" })).toBeVisible();
  await page.getByRole("link", { name: "Sound" }).click();
  await expect(page.getByRole("button", { name: "Low cut + mid boost" })).toBeVisible();
});

test("the language can be switched in the menu", async ({ page }) => {
  await page.goto("/#/");
  await page.getByRole("button", { name: "Menu" }).click();
  await page.getByRole("menuitemradio", { name: "Deutsch" }).click();
  await expect(page.getByRole("navigation", { name: "Bereiche" })).toBeVisible();
});
