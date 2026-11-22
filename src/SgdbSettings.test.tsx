import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen } from "@testing-library/react";
import { mockBackend } from "./test/tauriMock";
import { SgdbSettings } from "./SgdbSettings";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

describe("SteamGridDB settings", () => {
  it("checks and saves the user's key, then can remove it", async () => {
    const backend = mockBackend();
    render(<SgdbSettings t={t} />);
    fireEvent.change(await screen.findByPlaceholderText("Colle ta clé API SteamGridDB"), { target: { value: "  0123456789abcdef0123456789abcdef " } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Vérifier et enregistrer/ }));
    });
    expect(backend.calls.find((c) => c.cmd === "set_sgdb_api_key")?.args).toEqual({ key: "0123456789abcdef0123456789abcdef" });
    expect(await screen.findByText("Clé SteamGridDB enregistrée")).toBeTruthy();
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: "Retirer la clé" }));
    });
    expect(backend.count("clear_sgdb_api_key")).toBe(1);
    expect(screen.getByPlaceholderText("Colle ta clé API SteamGridDB")).toBeTruthy();
  });

  it("shows why a key was refused", async () => {
    mockBackend({ set_sgdb_api_key: () => { throw "Clé API SteamGridDB refusée"; } });
    render(<SgdbSettings t={t} />);
    fireEvent.change(await screen.findByPlaceholderText("Colle ta clé API SteamGridDB"), { target: { value: "0123456789abcdef0123456789abcdef" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Vérifier et enregistrer/ }));
    });
    expect((await screen.findByRole("alert")).textContent).toContain("refusée");
  });

  it("opens the page where SteamGridDB keys are made", async () => {
    const backend = mockBackend();
    render(<SgdbSettings t={t} />);
    await act(async () => {
      fireEvent.click(await screen.findByRole("button", { name: /Créer une clé/ }));
    });
    expect(backend.calls.some((c) => c.cmd === "plugin:opener|open_url" && JSON.stringify(c.args).includes("steamgriddb.com/profile/preferences/api"))).toBe(true);
  });
});
