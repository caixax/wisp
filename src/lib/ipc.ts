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
  AuthEvent,
  AuthState,
  CoverProgress,
  GameDetails,
  LibrarySnapshot,
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

export function authStatus(): Promise<AuthState> {
  return invoke<AuthState>('auth_status');
}

/**
 * Opens Epic's login page in its own window. Resolves as soon as the window
 * is up; the outcome arrives on {@link onAuthChanged}.
 */
export function authLogin(): Promise<void> {
  return invoke<void>('auth_login');
}

/** The URL to open in a normal browser for the manual flow. */
export function authLoginUrl(): Promise<string> {
  return invoke<string>('auth_login_url');
}

export function authSubmitCode(code: string): Promise<AuthState> {
  return invoke<AuthState>('auth_submit_code', { code });
}

export function authLogout(): Promise<AuthState> {
  return invoke<AuthState>('auth_logout');
}

export function onAuthChanged(handler: (event: AuthEvent) => void): Promise<UnlistenFn> {
  return listen<AuthEvent>('auth:changed', (event) => handler(event.payload));
}

/** The cached library. Cheap; this is what the grid renders on start. */
export function librarySnapshot(): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>('library_snapshot');
}

/** Re-reads the library from Legendary. `force` also refreshes its metadata. */
export function libraryRefresh(force = false): Promise<LibrarySnapshot> {
  return invoke<LibrarySnapshot>('library_refresh', { force });
}

export function gameDetails(appName: string): Promise<GameDetails | null> {
  return invoke<GameDetails | null>('game_details', { appName });
}

export function onCoverProgress(handler: (progress: CoverProgress) => void): Promise<UnlistenFn> {
  return listen<CoverProgress>('library:covers', (event) => handler(event.payload));
}

/** Opens one of Wisp's own folders in the system file manager. */
export function openPath(path: string): Promise<void> {
  return invoke<void>('open_path', { path });
}
