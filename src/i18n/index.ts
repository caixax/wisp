/**
 * i18n setup.
 *
 * Translations are bundled (they are a few KB) so the first paint never waits
 * on a network or disk read. The initial language comes from the backend:
 * the user's saved preference, else the operating system locale, else English.
 */

import i18n from 'i18next';
import { initReactI18next } from 'react-i18next';

import en from './locales/en.json';
import es from './locales/es.json';

export const SUPPORTED_LANGUAGES = [
  { code: 'en', label: 'English' },
  { code: 'es', label: 'Español' },
] as const;

export type LanguageCode = (typeof SUPPORTED_LANGUAGES)[number]['code'];

export const FALLBACK_LANGUAGE: LanguageCode = 'en';

function isSupported(code: string): code is LanguageCode {
  return SUPPORTED_LANGUAGES.some((language) => language.code === code);
}

/**
 * Maps any BCP-47 tag onto a language Wisp ships ("es-ES" -> "es"),
 * falling back to English for anything unknown.
 */
export function resolveLanguage(tag: string | null | undefined): LanguageCode {
  if (!tag) return FALLBACK_LANGUAGE;
  const base = tag.toLowerCase().split(/[-_]/)[0] ?? '';
  return isSupported(base) ? base : FALLBACK_LANGUAGE;
}

/** Human-readable name of a language tag, for the "system language" option. */
export function languageLabel(code: LanguageCode): string {
  return SUPPORTED_LANGUAGES.find((language) => language.code === code)?.label ?? code;
}

void i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    es: { translation: es },
  },
  lng: FALLBACK_LANGUAGE,
  fallbackLng: FALLBACK_LANGUAGE,
  interpolation: { escapeValue: false },
});

export default i18n;
