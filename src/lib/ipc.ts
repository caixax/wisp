/**
 * Typed wrappers around the Tauri commands.
 *
 * Components never call `invoke` directly: every backend call goes through a
 * named function here, which keeps command names and argument shapes in one
 * place and makes the surface easy to mock in tests.
 */

import { invoke } from '@tauri-apps/api/core';

import type { AppInfo, Settings, Theme } from './types';

export function getAppInfo(): Promise<AppInfo> {
  return invoke<AppInfo>('app_info');
}

export function getSettings(): Promise<Settings> {
  return invoke<Settings>('get_settings');
}

/** Pass `null` to go back to following the system language. */
export function setLanguage(language: string | null): Promise<Settings> {
  return invoke<Settings>('set_language', { language });
}

export function setTheme(theme: Theme): Promise<Settings> {
  return invoke<Settings>('set_theme', { theme });
}

export function setOnboardingCompleted(completed: boolean): Promise<Settings> {
  return invoke<Settings>('set_onboarding_completed', { completed });
}
