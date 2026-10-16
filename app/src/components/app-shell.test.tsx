import { screen, waitFor } from "@testing-library/react";
import { describe, expect, it } from "vitest";
import { renderApp } from "@/test/render";

describe("app shell", () => {
  it("without a microphone the controls are off and a hint shows", async () => {
    await renderApp("/", (b) => {
      b.state.problem = "notFound";
    });
    expect(await screen.findByText("Kein PD200X gefunden")).toBeInTheDocument();
    expect(screen.getByText("nicht verbunden")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Stummschalten" })).toBeDisabled();
  });

  it("explains the udev rule when access is denied", async () => {
    await renderApp("/", (b) => {
      b.state.problem = "permissionDenied";
    });
    expect(await screen.findByText("Keine Berechtigung für das Mikrofon")).toBeInTheDocument();
    expect(screen.getByText(/udevadm control --reload-rules/)).toBeInTheDocument();
  });

  it("the menu switches restoring on and shows the about dialog", async () => {
    const { backend, user } = await renderApp("/");
    await user.click(await screen.findByRole("button", { name: "Menü" }));
    await user.click(await screen.findByRole("menuitemcheckbox", { name: /beim Einstecken erneut senden/ }));
    await waitFor(() => expect(backend.state.snapshot.settings.restoreOnConnect).toBe(true));
    await user.click(screen.getByRole("button", { name: "Menü" }));
    await user.click(await screen.findByRole("menuitem", { name: "Über pd200x-control" }));
    expect(await screen.findByText(/pd200x von advnpzn/)).toBeInTheDocument();
  });

  it("navigates between the sections", async () => {
    const { user } = await renderApp("/");
    await user.click(await screen.findByRole("link", { name: "Licht" }));
    expect(await screen.findByText("Leuchtring")).toBeInTheDocument();
    await user.click(screen.getByRole("link", { name: "Klang" }));
    expect(await screen.findByText("Equalizer")).toBeInTheDocument();
  });
});
