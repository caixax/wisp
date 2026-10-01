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

export interface AuthState {
  loggedIn: boolean;
  account: string | null;
  gamesAvailable: number;
}

/** Payload of the `auth:changed` event: a new state, or a failure. */
export type AuthEvent = AuthState | { error: string };

export function isAuthFailure(event: AuthEvent): event is { error: string } {
  return 'error' in event;
}

export interface InstalledInfo {
  version: string;
  installPath: string;
  installSize: number;
  canRunOffline: boolean;
  needsVerification: boolean;
  platform: string;
  executable: string | null;
  savePath: string | null;
}

export interface GameSummary {
  appName: string;
  title: string;
  developer: string | null;
  availableVersion: string | null;
  installed: InstalledInfo | null;
  updateAvailable: boolean;
  hasCover: boolean;
  cloudSaves: boolean;
  /** Set for titles really managed by Origin, Ubisoft Connect and such. */
  thirdPartyStore: string | null;
  installable: boolean;
  dlcCount: number;
  installedDlcCount: number;
}

export interface DlcSummary {
  appName: string;
  title: string;
  availableVersion: string | null;
  installed: InstalledInfo | null;
  updateAvailable: boolean;
}

export interface GameDetails extends GameSummary {
  description: string | null;
  dlcs: DlcSummary[];
}

export interface LibrarySnapshot {
  games: GameSummary[];
  installedCount: number;
  updateCount: number;
  /** Unix seconds of the last successful refresh. */
  updatedAt: number | null;
  /** False before the first refresh ever ran. */
  fetched: boolean;
}

/** Payload of the `library:covers` event. */
export interface CoverProgress {
  done: number;
  total: number;
}

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
