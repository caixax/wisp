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

export interface RuntimeStatus {
  installed: boolean;
  executable: string;
  /** Version recorded when Wisp installed the binary. */
  version: string | null;
  /** False when GitHub published no checksum for the downloaded asset. */
  verified: boolean;
  /** Release asset this platform needs, or null on an unsupported CPU. */
  asset: string | null;
}

/** Result of actually running `legendary --version`. */
export interface RuntimeProbe {
  ok: boolean;
  version: string | null;
  error: string | null;
}

export interface UpdateCheck {
  current: string | null;
  latest: string;
  publishedAt: string | null;
  asset: string;
  size: number;
  updateAvailable: boolean;
}

/** Payload of the `runtime:progress` event. */
export type RuntimeProgress =
  | { phase: 'checking' }
  | { phase: 'downloading'; version: string; downloaded: number; total: number | null }
  | { phase: 'verifying' }
  | { phase: 'installing' }
  | { phase: 'done'; version: string }
  | { phase: 'failed'; message: string };

export interface AppInfo {
  version: string;
  platform: string;
  portable: boolean;
  paths: WispPaths;
  defaultInstallDir: string;
  systemLocale: string | null;
  settings: Settings;
  runtime: RuntimeStatus;
}

/** Discriminant of `Error::kind()` in `src-tauri/src/error.rs`. */
export type ErrorKind =
  | 'io'
  | 'serde'
  | 'paths'
  | 'config'
  | 'http'
  | 'checksum'
  | 'runtime'
  | 'legendary'
  | 'unsupported_arch'
  | 'unsupported_platform';

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
