/**
 * Mirrors of the Rust types exposed over IPC.
 *
 * Keep these in sync with `src-tauri/src/commands.rs`, `settings/mod.rs` and
 * `platform/mod.rs`. Rust serialises with `rename_all = "camelCase"`.
 */

export type Theme = 'dark' | 'light' | 'system';

export interface WispPaths {
  portable: boolean;
  root: string;
  config: string;
  runtime: string;
  legendary: string;
  cache: string;
  logs: string;
}

export interface Settings {
  /** BCP-47 tag, or `null` to follow the system language. */
  language: string | null;
  theme: Theme;
  onboardingCompleted: boolean;
}

export interface AppInfo {
  version: string;
  platform: string;
  portable: boolean;
  paths: WispPaths;
  defaultInstallDir: string;
  systemLocale: string | null;
  settings: Settings;
}

/** Discriminant of `Error::kind()` in `src-tauri/src/error.rs`. */
export type ErrorKind = 'io' | 'serde' | 'paths' | 'config' | 'unsupported_platform';

export interface WispError {
  kind: ErrorKind;
  message: string;
}

export function isWispError(value: unknown): value is WispError {
  return (
    typeof value === 'object' &&
    value !== null &&
    'kind' in value &&
    'message' in value &&
    typeof (value as WispError).message === 'string'
  );
}
