import { fireEvent, screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { lastChange, renderApp } from "@/test/render";

describe("Klang", () => {
  it("applies a preset", async () => {
    const { backend, user } = await renderApp("/sound");
    await user.click(await screen.findByRole("button", { name: "Natürlich" }));
    await waitFor(() => expect(backend.state.snapshot.settings.tone?.source).toBe("natural"));
    expect(await screen.findByRole("button", { name: "Natürlich", pressed: true })).toBeInTheDocument();
  });

  it("edits a band with the keyboard and in the fields", async () => {
    const { backend, user } = await renderApp("/sound", (b) => {
      b.state.snapshot.settings.tone = null;
    });
    const handle = await screen.findByRole("slider", { name: "Band 3" });
    handle.focus();
    await user.keyboard("{ArrowUp}");
    await waitFor(() =>
      expect(lastChange(backend)).toMatchObject({
        kind: "eqBand",
        value: { index: 2, band: { gainDb: 0.5, enabled: true } },
      }),
    );
    expect(await screen.findByText("Eigene Einstellung")).toBeInTheDocument();

    const freq = screen.getByLabelText("Frequenz (Hz)");
    await user.clear(freq);
    await user.type(freq, "2500{Enter}");
    await waitFor(() => expect(lastChange(backend)).toMatchObject({ value: { band: { frequency: 2500 } } }));

    const q = screen.getByLabelText("Güte (Q)");
    await user.clear(q);
    await user.type(q, "1,5{Enter}");
    await waitFor(() => expect(lastChange(backend)).toMatchObject({ value: { band: { q: 1.5 } } }));
  });

  it("the mouse wheel changes Q", async () => {
    const { backend } = await renderApp("/sound");
    const handle = await screen.findByRole("slider", { name: "Band 1" });
    fireEvent.wheel(handle, { deltaY: -100 });
    await waitFor(() => expect(lastChange(backend)).toMatchObject({ value: { index: 0 } }));
  });

  it("selects the high-pass, whose level cannot be changed", async () => {
    const { user } = await renderApp("/sound");
    await user.click(await screen.findByRole("tab", { name: /HP/ }));
    expect(screen.getByLabelText("Pegel (dB)")).toBeDisabled();
    expect(screen.getAllByText("Hochpass").length).toBeGreaterThan(0);
  });

  it("saves, renames and deletes an own profile", async () => {
    const { backend, user } = await renderApp("/sound");
    await user.click(await screen.findByRole("button", { name: "Eigenes Profil" }));
    await user.type(screen.getByLabelText("Name"), "Podcast");
    await user.click(screen.getByRole("button", { name: "Speichern" }));
    expect(await screen.findByRole("button", { name: "Podcast", pressed: true })).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Podcast umbenennen" }));
    const dialog = await screen.findByRole("dialog");
    const name = within(dialog).getByLabelText("Name");
    await user.clear(name);
    await user.type(name, "Talk{Enter}");
    await waitFor(() => expect(backend.state.snapshot.settings.profiles?.[0].name).toBe("Talk"));

    await user.click(await screen.findByRole("button", { name: "Talk löschen" }));
    await waitFor(() => expect(backend.state.snapshot.settings.profiles).toHaveLength(0));
  });

  it("switches the compressor on", async () => {
    const { backend, user } = await renderApp("/sound");
    await user.click(await screen.findByRole("switch", { name: "Kompressor" }));
    expect(lastChange(backend)).toEqual({ kind: "compressor", value: { enabled: true, thresholdDb: -20 } });
  });
});
