import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { BootError, BootScreen } from '@/components/BootScreen';
import { RuntimeBanner } from '@/components/RuntimeBanner';
import { Sidebar } from '@/components/Sidebar';
import { useAuth } from '@/hooks/useAuth';
import { useLibrary } from '@/hooks/useLibrary';
import { useRuntime } from '@/hooks/useRuntime';
import { useRuntimeInstall } from '@/hooks/useRuntimeInstall';
import { useTheme } from '@/hooks/useTheme';
import { resolveLanguage } from '@/i18n';
import * as ipc from '@/lib/ipc';
import { DEFAULT_ROUTE, type Route } from '@/lib/routes';
import { isWispError, type AppInfo, type Settings, type Theme } from '@/lib/types';
import { DownloadsPage } from '@/pages/DownloadsPage';
import { GamePage } from '@/pages/GamePage';
import { LibraryPage } from '@/pages/LibraryPage';
import { RuntimeSetupPage } from '@/pages/RuntimeSetupPage';
import { SettingsPage } from '@/pages/SettingsPage';

import styles from './App.module.css';

type BootState =
  | { status: 'loading' }
  | { status: 'ready'; info: AppInfo }
  | { status: 'failed'; message: string };

export default function App() {
  const [boot, setBoot] = useState<BootState>({ status: 'loading' });

  const load = useCallback(() => {
    ipc
      .getAppInfo()
      .then((info) => setBoot({ status: 'ready', info }))
      .catch((error: unknown) => {
        setBoot({
          status: 'failed',
          message: isWispError(error) ? error.message : String(error),
        });
      });
  }, []);

  useEffect(() => {
    load();
  }, [load]);

  const retry = useCallback(() => {
    setBoot({ status: 'loading' });
    load();
  }, [load]);

  if (boot.status === 'loading') return <BootScreen />;
  if (boot.status === 'failed') return <BootError message={boot.message} onRetry={retry} />;

  return <Shell info={boot.info} />;
}

function Shell({ info }: { info: AppInfo }) {
  const { i18n } = useTranslation();
  const [settings, setSettings] = useState<Settings>(info.settings);
  const [route, setRoute] = useState<Route>(DEFAULT_ROUTE);
  const [updateDismissed, setUpdateDismissed] = useState(false);

  const runtime = useRuntime(info.runtime);
  const { runProbe, checkUpdate, applyInstalled, status: runtimeStatus } = runtime;
  const { state: installState, run: install } = useRuntimeInstall(applyInstalled);

  const auth = useAuth(runtimeStatus.installed);
  const library = useLibrary(auth.phase === 'signedIn');

  useTheme(settings.theme);

  useEffect(() => {
    // An explicit preference wins; otherwise follow the OS locale.
    void i18n.changeLanguage(resolveLanguage(settings.language ?? info.systemLocale));
  }, [i18n, settings.language, info.systemLocale]);

  // Confirm Legendary runs and look for a newer release, once the window is
  // already up. Neither blocks the UI and a failed update check stays quiet.
  useEffect(() => {
    if (!runtimeStatus.installed) return;
    void runProbe();
    void checkUpdate();
  }, [runtimeStatus.installed, runProbe, checkUpdate]);

  const handleLanguageChange = useCallback((language: string | null) => {
    void ipc.setLanguage(language).then(setSettings);
  }, []);

  const handleThemeChange = useCallback((theme: Theme) => {
    void ipc.setTheme(theme).then(setSettings);
  }, []);

  const openGame = useCallback((appName: string) => setRoute({ page: 'game', appName }), []);
  const backToLibrary = useCallback(() => setRoute({ page: 'library' }), []);

  if (!runtimeStatus.installed) {
    return <RuntimeSetupPage onInstalled={applyInstalled} />;
  }

  return (
    <div className={styles.shell}>
      <Sidebar current={route} onNavigate={setRoute} version={info.version} />
      <div className={styles.main}>
        <RuntimeBanner
          probe={runtime.probe}
          update={updateDismissed ? null : runtime.update}
          busy={installState.kind === 'running'}
          onRepair={() => void install(true)}
          onUpdate={() => void install(false)}
          onDismissUpdate={() => setUpdateDismissed(true)}
        />
        {route.page === 'library' ? (
          <LibraryPage auth={auth} library={library} onOpenGame={openGame} />
        ) : null}
        {route.page === 'game' ? (
          <GamePage
            appName={route.appName}
            coverVersion={library.coverVersion}
            onBack={backToLibrary}
          />
        ) : null}
        {route.page === 'downloads' ? <DownloadsPage /> : null}
        {route.page === 'settings' ? (
          <SettingsPage
            info={info}
            settings={settings}
            runtime={runtime}
            auth={auth}
            onLanguageChange={handleLanguageChange}
            onThemeChange={handleThemeChange}
          />
        ) : null}
      </div>
    </div>
  );
}
