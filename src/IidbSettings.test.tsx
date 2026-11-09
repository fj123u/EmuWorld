import { describe, expect, it } from "vitest";
import { act, fireEvent, render, screen, waitFor } from "@testing-library/react";
import { mockBackend } from "./test/tauriMock";
import { IidbSettings } from "./IidbSettings";
import fr from "./i18n/fr";

const t = (key: string, params: Record<string, string | number> = {}) => {
  let v = key.split(".").reduce<unknown>((o, k) => (o as Record<string, unknown>)?.[k], fr);
  if (typeof v !== "string") return key;
  for (const [k, val] of Object.entries(params)) v = (v as string).replace(`{${k}}`, String(val));
  return v as string;
};

describe("iiDB settings", () => {
  it("saves the user's own key and shows the connection", async () => {
    const backend = mockBackend();
    render(<IidbSettings t={t} />);
    const input = await screen.findByLabelText("Clé API iiDB (facultative)");
    expect((input as HTMLInputElement).type, "the key is never shown in clear").toBe("password");
    fireEvent.change(input, { target: { value: "  iidb_test_0123456789abcdef  " } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Vérifier et enregistrer/ }));
    });
    expect(backend.calls.find((c) => c.cmd === "set_iidb_api_key")?.args).toEqual({ key: "iidb_test_0123456789abcdef" });
    expect(await screen.findByText(/Connecté à iiDB \(fj123u, palier standard, 1 req\/s\)/)).toBeTruthy();
  });

  it("shows why a key was refused", async () => {
    mockBackend({ set_iidb_api_key: () => { throw "Clé API iiDB refusée : vérifie-la sur accounts.iisu.network"; } });
    render(<IidbSettings t={t} />);
    fireEvent.change(await screen.findByLabelText("Clé API iiDB (facultative)"), { target: { value: "wrong_key_0123456789" } });
    await act(async () => {
      fireEvent.click(screen.getByRole("button", { name: /Vérifier et enregistrer/ }));
    });
    expect(screen.getByRole("alert").textContent).toContain("refusée");
  });

  it("lets the user remove the key", async () => {
    const backend = mockBackend({ get_iidb_status: () => ({ configured: true, username: "fj123u", tier: "standard", requests_per_second: 1 }) });
    render(<IidbSettings t={t} />);
    const remove = await screen.findByRole("button", { name: "Retirer la clé" });
    await act(async () => {
      fireEvent.click(remove);
    });
    expect(backend.count("clear_iidb_api_key")).toBe(1);
    expect(screen.getByLabelText("Clé API iiDB (facultative)")).toBeTruthy();
  });

  it("credits iiDB with a link to its site, as its terms require", async () => {
    const backend = mockBackend();
    render(<IidbSettings t={t} />);
    expect(screen.getByText(/Covers fournies par/)).toBeTruthy();
    fireEvent.click(screen.getByRole("button", { name: "iiDB" }));
    await waitFor(() => expect(backend.calls.some((c) => c.cmd === "plugin:opener|open_url" && JSON.stringify(c.args).includes("https://iidb.iisu.network"))).toBe(true));
  });
});
