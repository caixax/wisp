import { useTranslation } from 'react-i18next';

import { useRuntimeInstall } from '@/hooks/useRuntimeInstall';
import { formatBytes, formatDate } from '@/lib/format';
import * as ipc from '@/lib/ipc';
import type { RuntimeProbe, RuntimeStatus, UpdateCheck } from '@/lib/types';

import { Button } from './Button';
import { ProgressBar } from './ProgressBar';

import styles from './RuntimeSection.module.css';

interface RuntimeSectionProps {
  status: RuntimeStatus;
  probe: RuntimeProbe | null;
  update: UpdateCheck | null;
  updateError: string | null;
  checking: boolean;
  onCheckUpdate: () => void;
  onInstalled: (status: RuntimeStatus) => void;
}

/**
 * Settings panel for the Legendary runtime: what is installed, whether it
 * runs, and the two actions that can change that (update and repair).
 */
export function RuntimeSection({
  status,
  probe,
  update,
  updateError,
  checking,
  onCheckUpdate,
  onInstalled,
}: RuntimeSectionProps) {
  const { t, i18n } = useTranslation();
  const { state, run } = useRuntimeInstall(onInstalled);

  const busy = state.kind === 'running' || checking;
  const version = probe?.version ?? status.version;
  const published = formatDate(update?.publishedAt ?? null, i18n.language);

  return (
    <section className={styles.section}>
      <h2 className={styles.sectionTitle}>{t('runtime.section')}</h2>

      <dl className={styles.facts}>
        <Fact
          label={t('runtime.version')}
          value={version ?? t('runtime.notInstalled')}
          tone={status.installed ? 'normal' : 'muted'}
        />
        <Fact
          label={t('runtime.checksum')}
          value={status.verified ? t('runtime.checksumVerified') : t('runtime.checksumUnverified')}
          tone={status.verified ? 'normal' : 'muted'}
        />
        <Fact label={t('runtime.location')} value={status.executable} mono />
      </dl>

      {state.kind === 'running' ? (
        <div className={styles.progress}>
          <ProgressBar
            value={
              state.progress.phase === 'downloading' && state.progress.total
                ? state.progress.downloaded / state.progress.total
                : null
            }
            label={t('runtime.phaseInstalling')}
          />
          <span className={styles.progressLabel}>
            {state.progress.phase === 'downloading'
              ? `${formatBytes(state.progress.downloaded, i18n.language)}${
                  state.progress.total
                    ? ` / ${formatBytes(state.progress.total, i18n.language)}`
                    : ''
                }`
              : t('runtime.phaseVerifying')}
          </span>
        </div>
      ) : null}

      <div className={styles.actions}>
        <Button onClick={onCheckUpdate} disabled={busy}>
          {checking ? t('runtime.checking') : t('runtime.checkUpdate')}
        </Button>
        {update?.updateAvailable ? (
          <Button variant="primary" onClick={() => void run(false)} disabled={busy}>
            {t('runtime.updateTo', { version: update.latest })}
          </Button>
        ) : null}
        <Button onClick={() => void run(true)} disabled={busy}>
          {t('runtime.repair')}
        </Button>
        <Button variant="ghost" onClick={() => void ipc.openPath(status.executable)}>
          {t('runtime.openFolder')}
        </Button>
      </div>

      <p className={styles.hint}>
        {update && !update.updateAvailable
          ? t('runtime.upToDate')
          : update?.updateAvailable
            ? t('runtime.releaseInfo', {
                size: formatBytes(update.size, i18n.language),
                date: published ?? '?',
              })
            : t('runtime.repairHint')}
      </p>

      {updateError ? (
        <p className={styles.error} role="status">
          {t('runtime.checkFailed', { message: updateError })}
        </p>
      ) : null}

      {state.kind === 'failed' ? (
        <p className={styles.error} role="alert">
          {state.message}
        </p>
      ) : null}
    </section>
  );
}

function Fact({
  label,
  value,
  mono = false,
  tone = 'normal',
}: {
  label: string;
  value: string;
  mono?: boolean;
  tone?: 'normal' | 'muted';
}) {
  return (
    <div className={styles.fact}>
      <dt className={styles.factLabel}>{label}</dt>
      <dd className={styles.factValue} data-mono={mono} data-tone={tone}>
        {value}
      </dd>
    </div>
  );
}
