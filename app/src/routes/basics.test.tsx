import { screen, waitFor, within } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { lastChange, renderApp } from "@/test/render";

describe("Grundlagen", () => {
  it("shows the live values and warns at maximum gain", async () => {
    await renderApp("/");
    expect(
      await screen.findByText("Hardware am Maximum. Mehr geht nur über die Mikrofon-Lautstärke des Systems."),
    ).toBeInTheDocument();
    expect(screen.getByRole("slider", { name: "Mikrofon-Gain" })).toHaveAttribute("aria-valuenow", "100");
    expect(screen.getByRole("slider", { name: "Lautstärke" })).toHaveAttribute("aria-valuenow", "41");
    expect(screen.getByText("verbunden")).toBeInTheDocument();
  });

  it("mutes and unmutes", async () => {
    const { backend, user } = await renderApp("/");
    await user.click(await screen.findByRole("button", { name: "Stummschalten" }));
    expect(lastChange(backend)).toEqual({ kind: "mute", value: true });
    expect(await screen.findByRole("button", { name: "Stummgeschaltet" })).toHaveAttribute("aria-pressed", "true");
  });

  it("moves the gain with the keyboard", async () => {
    const { backend, user } = await renderApp("/", (b) => {
      b.state.snapshot.live!.gain = 70;
    });
    const gain = await screen.findByRole("slider", { name: "Mikrofon-Gain" });
    await waitFor(() => expect(gain).toHaveAttribute("aria-valuenow", "70"));
    gain.focus();
    await user.keyboard("{ArrowRight}");
    await waitFor(() => expect(lastChange(backend)).toEqual({ kind: "gain", value: 71 }));
  });

  it("switches the system volume boost on with the remembered value", async () => {
    const { backend, user } = await renderApp("/", (b) => {
      b.state.snapshot.settings.boostDb = 14.6;
    });
    await user.click(await screen.findByRole("switch", { name: "Mikrofon-Lautstärke über 100 % an" }));
    await waitFor(() => expect(backend.state.snapshot.boostDb).toBe(14.6));
    expect(await screen.findByText("175 %")).toBeInTheDocument();
    expect(screen.getByText("+14,6 dB")).toBeInTheDocument();
  });

  it("toggles monitoring and noise cancellation", async () => {
    const { backend, user } = await renderApp("/");
    await user.click(await screen.findByRole("switch", { name: "Mithören" }));
    expect(lastChange(backend)).toEqual({ kind: "monitor", value: false });
    await user.click(screen.getByRole("switch", { name: "Rauschunterdrückung" }));
    expect(lastChange(backend)).toEqual({ kind: "noiseCancellation", value: true });
  });

  it("calibrates and applies the proposal", async () => {
    const { backend, user } = await renderApp("/");
    await user.click(await screen.findByRole("button", { name: "Pegel einmessen" }));
    await user.click(screen.getByRole("button", { name: "Los geht's" }));
    const dialog = await screen.findByRole("dialog");
    expect(await within(dialog).findByText("162 %")).toBeInTheDocument();
    expect(within(dialog).getByText("+12,5 dB")).toBeInTheDocument();
    expect(within(dialog).getByText("−9,0 dB")).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Übernehmen" }));
    await waitFor(() => expect(backend.state.snapshot.boostDb).toBe(12.5));
  });

  it("explains a failed calibration", async () => {
    const { user } = await renderApp("/", (b) => {
      b.state.voiceDb = -58;
    });
    await user.click(await screen.findByRole("button", { name: "Pegel einmessen" }));
    await user.click(screen.getByRole("button", { name: "Los geht's" }));
    expect(await screen.findByText(/Keine Stimme gehört/)).toBeInTheDocument();
  });
});

describe("protection from apps", () => {
  it("asks first, then switches it on", async () => {
    const { backend, user } = await renderApp("/");
    await user.click(await screen.findByRole("switch", { name: "Vor Apps schützen" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText(/einmal neu, sonst finden sie das Mikrofon nicht/)).toBeInTheDocument();
    await user.click(within(dialog).getByRole("button", { name: "Abbrechen" }));
    expect(backend.state.snapshot.protected).toBe(false);
    await user.click(screen.getByRole("switch", { name: "Vor Apps schützen" }));
    await user.click(within(await screen.findByRole("dialog")).getByRole("button", { name: "Einschalten" }));
    await waitFor(() => expect(backend.state.snapshot.protected).toBe(true));
    expect(backend.state.calls.some((c) => c.cmd === "set_protected" && c.args.on === true)).toBe(true);
  });

  it("names the restart command in the Flatpak", async () => {
    const { user } = await renderApp("/", (b) => {
      b.state.snapshot.sandboxed = true;
    });
    await user.click(await screen.findByRole("switch", { name: "Vor Apps schützen" }));
    const dialog = await screen.findByRole("dialog");
    expect(within(dialog).getByText("systemctl --user restart pipewire-pulse")).toBeInTheDocument();
    expect(within(dialog).queryByText(/Beim Umschalten setzt der Ton kurz aus/)).not.toBeInTheDocument();
  });
});
