import { useCallback, useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { BootError, BootScreen } from '@/components/BootScreen';
import { Sidebar } from '@/components/Sidebar';
import { useTheme } from '@/hooks/useTheme';
import { resolveLanguage } from '@/i18n';
import * as ipc from '@/lib/ipc';
import { DEFAULT_ROUTE, type Route } from '@/lib/routes';
import { isWispError, type AppInfo, type Settings, type Theme } from '@/lib/types';
import { DownloadsPage } from '@/pages/DownloadsPage';
import { LibraryPage } from '@/pages/LibraryPage';
import { SettingsPage } from '@/pages/SettingsPage';

import styles from './App.module.css';

type BootState =
  | { status: 'loading' }
  | { status: 'ready'; info: AppInfo }
  | { status: 'failed'; message: string };

export default function App() {
  const { i18n } = useTranslation();
  const [boot, setBoot] = useState<BootState>({ status: 'loading' });
  const [settings, setSettings] = useState<Settings | null>(null);
  const [route, setRoute] = useState<Route>(DEFAULT_ROUTE);

  const load = useCallback(() => {
    ipc
      .getAppInfo()
      .then((info) => {
        setBoot({ status: 'ready', info });
        setSettings(info.settings);
        // An explicit preference wins; otherwise follow the OS locale.
        void i18n.changeLanguage(resolveLanguage(info.settings.language ?? info.systemLocale));
      })
      .catch((error: unknown) => {
        setBoot({
          status: 'failed',
          message: isWispError(error) ? error.message : String(error),
        });
      });
  }, [i18n]);

  useEffect(() => {
    load();
  }, [load]);

  const retry = useCallback(() => {
    setBoot({ status: 'loading' });
    load();
  }, [load]);

  useTheme(settings?.theme ?? 'dark');

  const handleLanguageChange = useCallback(
    (language: string | null) => {
      void ipc.setLanguage(language).then((next) => {
        setSettings(next);
        const systemLocale = boot.status === 'ready' ? boot.info.systemLocale : null;
        void i18n.changeLanguage(resolveLanguage(next.language ?? systemLocale));
      });
    },
    [boot, i18n],
  );

  const handleThemeChange = useCallback((theme: Theme) => {
    void ipc.setTheme(theme).then(setSettings);
  }, []);

  if (boot.status === 'loading') return <BootScreen />;
  if (boot.status === 'failed') return <BootError message={boot.message} onRetry={retry} />;

  return (
    <div className={styles.shell}>
      <Sidebar current={route} onNavigate={setRoute} version={boot.info.version} />
      {route.page === 'library' || route.page === 'game' ? <LibraryPage /> : null}
      {route.page === 'downloads' ? <DownloadsPage /> : null}
      {route.page === 'settings' && settings ? (
        <SettingsPage
          info={boot.info}
          settings={settings}
          onLanguageChange={handleLanguageChange}
          onThemeChange={handleThemeChange}
        />
      ) : null}
    </div>
  );
}
