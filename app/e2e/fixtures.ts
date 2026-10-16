import { test as base, expect, type Page } from "@playwright/test";

/** Changes the mock backend's state in the page. */
export async function mock(page: Page, fn: string) {
  await page.evaluate((f) => {
    const state = (window as unknown as { __pd200xMock: { state: Record<string, unknown> } }).__pd200xMock.state;
    new Function("state", f)(state);
  }, fn);
}

export async function mockState<T>(page: Page, path: string): Promise<T> {
  return page.evaluate((p) => {
    let v: unknown = (window as unknown as { __pd200xMock: { state: unknown } }).__pd200xMock.state;
    for (const k of p.split(".")) v = (v as Record<string, unknown>)?.[k];
    return v;
  }, path) as Promise<T>;
}

export const test = base.extend<{ app: Page }>({
  app: async ({ page }, use) => {
    const errors: string[] = [];
    page.on("pageerror", (e) => errors.push(String(e)));
    await page.goto("/#/");
    await expect(page.getByRole("navigation", { name: "Bereiche" })).toBeVisible();
    await use(page);
    expect(errors, "uncaught errors in the page").toEqual([]);
  },
});

export { expect };
