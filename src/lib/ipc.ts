/**
 * Typed wrappers around the Tauri commands and events.
 *
 * Components never call `invoke` directly: every backend call goes through a
 * named function here, which keeps command names and argument shapes in one
 * place and makes the surface easy to mock in tests.
 */

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type {
  AppInfo,
  RuntimeProbe,
  RuntimeProgress,
  RuntimeStatus,
  Settings,
  Theme,
  UpdateCheck,
} from './types';

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

/** Cheap: reads a metadata file, never spawns Legendary. */
export function runtimeStatus(): Promise<RuntimeStatus> {
  return invoke<RuntimeStatus>('runtime_status');
}

/** Runs `legendary --version`. Costs around a second. */
export function runtimeProbe(): Promise<RuntimeProbe> {
  return invoke<RuntimeProbe>('runtime_probe');
}

export function runtimeCheckUpdate(): Promise<UpdateCheck> {
  return invoke<UpdateCheck>('runtime_check_update');
}

/**
 * Installs or repairs the Legendary runtime. Progress arrives on the
 * `runtime:progress` event; subscribe with {@link onRuntimeProgress} first.
 */
export function runtimeInstall(force = false): Promise<RuntimeStatus> {
  return invoke<RuntimeStatus>('runtime_install', { force });
}

export function onRuntimeProgress(
  handler: (progress: RuntimeProgress) => void,
): Promise<UnlistenFn> {
  return listen<RuntimeProgress>('runtime:progress', (event) => handler(event.payload));
}

/** Opens one of Wisp's own folders in the system file manager. */
export function openPath(path: string): Promise<void> {
  return invoke<void>('open_path', { path });
}
