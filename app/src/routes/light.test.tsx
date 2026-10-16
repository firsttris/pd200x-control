import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { lastChange, renderApp } from "@/test/render";

describe("Licht", () => {
  it("picks a colour and shows it in the preview", async () => {
    const { backend, user } = await renderApp("/light");
    await user.click(await screen.findByRole("button", { name: "Blau" }));
    expect(lastChange(backend)).toEqual({ kind: "rgbColor", value: "blue" });
    expect(await screen.findByText("Vorschau · Blau · 70 %")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Blau" })).toHaveAttribute("aria-pressed", "true");
  });

  it("switches the ring off and links it to mute", async () => {
    const { backend, user } = await renderApp("/light");
    await user.click(await screen.findByRole("switch", { name: "Licht aus bei Stumm" }));
    expect(lastChange(backend)).toEqual({ kind: "linkage", value: true });
    await user.click(screen.getByRole("switch", { name: "Leuchtring" }));
    expect(lastChange(backend)).toEqual({ kind: "rgbEnabled", value: false });
    expect(await screen.findByText("Vorschau · Ring aus")).toBeInTheDocument();
  });

  it("brightness with the keyboard", async () => {
    const { backend, user } = await renderApp("/light");
    const slider = await screen.findByRole("slider", { name: "Helligkeit" });
    slider.focus();
    await user.keyboard("{ArrowLeft}");
    await waitFor(() => expect(lastChange(backend)).toEqual({ kind: "rgbBrightness", value: 69 }));
  });
});
