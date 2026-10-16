import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp } from "@/test/render";

describe("Probeaufnahme", () => {
  it("records, shows peak and average, plays and stops", async () => {
    const { backend, user } = await renderApp("/");
    expect(await screen.findByText("Noch keine Aufnahme")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Abspielen" })).toBeDisabled();

    await user.click(screen.getByRole("button", { name: "Aufnehmen" }));
    await screen.findByRole("button", { name: "Stopp" });
    expect(backend.state.recording.state).toBe("recording");
    await user.click(screen.getByRole("button", { name: "Stopp" }));

    expect(await screen.findByText("−21,5 dB")).toBeInTheDocument();
    expect(screen.getByText("−37,5 dB")).toBeInTheDocument();
    expect(screen.getByText("Eher leise – „Pegel einmessen“ oben hilft.")).toBeInTheDocument();

    await user.click(screen.getByRole("button", { name: "Abspielen" }));
    await user.click(await screen.findByRole("button", { name: "Wiedergabe stoppen" }));
    await waitFor(() => expect(backend.state.recording.state).toBe("ready"));
    expect(await screen.findByRole("button", { name: "Neu aufnehmen" })).toBeInTheDocument();
  });

  it("judges loud and good recordings", async () => {
    const { backend, user } = await renderApp("/", (b) => {
      b.state.voiceDb = -9;
    });
    await user.click(await screen.findByRole("button", { name: "Aufnehmen" }));
    await user.click(await screen.findByRole("button", { name: "Stopp" }));
    expect(await screen.findByText("Gut ausgesteuert.")).toBeInTheDocument();
    backend.state.voiceDb = -1;
    await user.click(screen.getByRole("button", { name: "Neu aufnehmen" }));
    await user.click(await screen.findByRole("button", { name: "Stopp" }));
    expect(await screen.findByText("Sehr laut – es kann übersteuern.")).toBeInTheDocument();
  });

  it("cannot record without a microphone", async () => {
    await renderApp("/", (b) => {
      b.state.problem = "notFound";
    });
    expect(await screen.findByRole("button", { name: "Aufnehmen" })).toBeDisabled();
  });
});
