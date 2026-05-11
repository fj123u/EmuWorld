import { useState, useCallback, useEffect } from "react";
import fr, { type Translations } from "./fr";
import en from "./en";

export type Locale = "fr" | "en";

const translations: Record<Locale, Translations> = { fr, en };

const STORAGE_KEY = "emuworld-locale";

function detectLocale(): Locale {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored === "fr" || stored === "en") return stored;
  const nav = navigator.language.toLowerCase();
  if (nav.startsWith("fr")) return "fr";
  return "en";
}

type NestedKeyOf<T> = T extends object
  ? { [K in keyof T & string]: T[K] extends object ? `${K}.${NestedKeyOf<T[K]>}` : K }[keyof T & string]
  : never;

type TranslationKey = NestedKeyOf<Translations>;

function getNestedValue(obj: Record<string, unknown>, path: string): string {
  const parts = path.split(".");
  let current: unknown = obj;
  for (const part of parts) {
    if (current === null || current === undefined || typeof current !== "object") return path;
    current = (current as Record<string, unknown>)[part];
  }
  return typeof current === "string" ? current : path;
}

export function useTranslation() {
  const [locale, setLocaleState] = useState<Locale>(detectLocale);

  useEffect(() => {
    localStorage.setItem(STORAGE_KEY, locale);
  }, [locale]);

  const setLocale = useCallback((l: Locale) => {
    setLocaleState(l);
    localStorage.setItem(STORAGE_KEY, l);
  }, []);

  const t = useCallback(
    (key: string, params?: Record<string, string | number>): string => {
      let value = getNestedValue(translations[locale] as unknown as Record<string, unknown>, key);
      if (params) {
        Object.entries(params).forEach(([k, v]) => {
          value = value.replace(`{${k}}`, String(v));
        });
      }
      return value;
    },
    [locale]
  );

  return { t, locale, setLocale };
}

export type { Translations };
export { fr, en };
