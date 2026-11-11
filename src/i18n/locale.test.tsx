import { describe, expect, it } from "vitest";
import { act, render, screen } from "@testing-library/react";
import { useTranslation } from "./index";

function Label() {
  const { t } = useTranslation();
  return <span>{t("card.download")}</span>;
}

function Switcher() {
  const { setLocale } = useTranslation();
  return <button onClick={() => setLocale("en")}>en</button>;
}

describe("language change", () => {
  it("reaches every component that translates, not only the one that changed it", () => {
    localStorage.setItem("emuworld-locale", "fr");
    render(<><Label /><Switcher /></>);
    expect(screen.getByText("Télécharger")).toBeTruthy();
    act(() => { screen.getByRole("button", { name: "en" }).click(); });
    expect(screen.getByText("Download")).toBeTruthy();
  });
});
