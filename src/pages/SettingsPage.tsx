import { useTranslation } from 'react-i18next';

import { Page } from '@/components/Page';
import { languageLabel, resolveLanguage, SUPPORTED_LANGUAGES } from '@/i18n';
import type { AppInfo, Settings, Theme } from '@/lib/types';

import styles from './SettingsPage.module.css';

const THEMES: Theme[] = ['dark', 'light', 'system'];

interface SettingsPageProps {
  info: AppInfo;
  settings: Settings;
  onLanguageChange: (language: string | null) => void;
  onThemeChange: (theme: Theme) => void;
}

export function SettingsPage({
  info,
  settings,
  onLanguageChange,
  onThemeChange,
}: SettingsPageProps) {
  const { t } = useTranslation();
  const systemLanguage = languageLabel(resolveLanguage(info.systemLocale));

  return (
    <Page title={t('settings.title')} subtitle={t('settings.subtitle')}>
      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>{t('settings.appearance')}</h2>

        <div className={styles.field}>
          <label className={styles.label} htmlFor="language">
            {t('settings.language')}
          </label>
          <select
            id="language"
            className={styles.select}
            value={settings.language ?? ''}
            onChange={(event) =>
              onLanguageChange(event.target.value === '' ? null : event.target.value)
            }
          >
            <option value="">{t('settings.languageSystem', { language: systemLanguage })}</option>
            {SUPPORTED_LANGUAGES.map((language) => (
              <option key={language.code} value={language.code}>
                {language.label}
              </option>
            ))}
          </select>
          <p className={styles.hint}>{t('settings.languageHint')}</p>
        </div>

        <fieldset className={styles.field}>
          <legend className={styles.label}>{t('settings.theme')}</legend>
          <div className={styles.segmented}>
            {THEMES.map((theme) => (
              <label key={theme} className={styles.segment} data-active={settings.theme === theme}>
                <input
                  type="radio"
                  name="theme"
                  value={theme}
                  checked={settings.theme === theme}
                  onChange={() => onThemeChange(theme)}
                />
                {t(`settings.theme${theme[0]?.toUpperCase()}${theme.slice(1)}`)}
              </label>
            ))}
          </div>
        </fieldset>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>{t('settings.about')}</h2>
        <dl className={styles.facts}>
          <Fact label={t('settings.version')} value={info.version} />
          <Fact label={t('settings.platform')} value={info.platform} />
          <Fact
            label={t('settings.mode')}
            value={info.portable ? t('settings.modePortable') : t('settings.modeInstalled')}
          />
        </dl>
      </section>

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>{t('settings.paths')}</h2>
        <dl className={styles.facts}>
          <Fact label={t('settings.pathConfig')} value={info.paths.config} mono />
          <Fact label={t('settings.pathRuntime')} value={info.paths.runtime} mono />
          <Fact label={t('settings.pathLegendary')} value={info.paths.legendary} mono />
          <Fact label={t('settings.pathCache')} value={info.paths.cache} mono />
          <Fact label={t('settings.pathLogs')} value={info.paths.logs} mono />
          <Fact label={t('settings.pathInstall')} value={info.defaultInstallDir} mono />
        </dl>
      </section>
    </Page>
  );
}

function Fact({ label, value, mono = false }: { label: string; value: string; mono?: boolean }) {
  return (
    <div className={styles.fact}>
      <dt className={styles.factLabel}>{label}</dt>
      <dd className={styles.factValue} data-mono={mono}>
        {value}
      </dd>
    </div>
  );
}
