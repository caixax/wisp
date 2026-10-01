import { useTranslation } from 'react-i18next';

import { Button } from '@/components/Button';
import { ProgressBar } from '@/components/ProgressBar';
import { WispMark } from '@/components/WispMark';
import { useRuntimeInstall } from '@/hooks/useRuntimeInstall';
import { formatBytes } from '@/lib/format';
import type { RuntimeProgress, RuntimeStatus } from '@/lib/types';

import styles from './RuntimeSetupPage.module.css';

/** Rough size of the Legendary binary, shown before the download starts. */
const APPROXIMATE_SIZE = 17 * 1024 * 1024;

interface RuntimeSetupPageProps {
  onInstalled: (status: RuntimeStatus) => void;
}

/**
 * First-run screen. Wisp cannot do anything useful without Legendary, so this
 * stands in front of the app until the binary is in place.
 */
export function RuntimeSetupPage({ onInstalled }: RuntimeSetupPageProps) {
  const { t, i18n } = useTranslation();
  const { state, run } = useRuntimeInstall(onInstalled);

  const busy = state.kind === 'running';

  return (
    <main className={styles.screen}>
      <div className={styles.card}>
        <WispMark className={styles.mark} />

        <h1 className={styles.title}>{t('runtime.setupTitle')}</h1>
        <p className={styles.body}>
          {t('runtime.setupIntro', { size: formatBytes(APPROXIMATE_SIZE, i18n.language) })}
        </p>
        <p className={styles.note}>{t('runtime.setupDetails')}</p>

        {busy ? (
          <ProgressView progress={state.progress} />
        ) : (
          <Button variant="primary" onClick={() => void run()} className={styles.action}>
            {state.kind === 'failed' ? t('runtime.retry') : t('runtime.setupAction')}
          </Button>
        )}

        {state.kind === 'failed' ? (
          <div className={styles.error} role="alert">
            <strong>{t('runtime.failed')}</strong>
            <span>{state.message}</span>
          </div>
        ) : null}
      </div>
    </main>
  );
}

function ProgressView({ progress }: { progress: RuntimeProgress }) {
  const { t, i18n } = useTranslation();

  const fraction =
    progress.phase === 'downloading' && progress.total
      ? progress.downloaded / progress.total
      : null;
  const [key, params] = phaseKey(progress);
  const label = t(key, params);

  return (
    <div className={styles.progress}>
      <ProgressBar value={fraction} label={label} />
      <div className={styles.progressRow}>
        <span>{label}</span>
        {progress.phase === 'downloading' ? (
          <span className={styles.bytes}>
            {formatBytes(progress.downloaded, i18n.language)}
            {progress.total ? ` / ${formatBytes(progress.total, i18n.language)}` : ''}
          </span>
        ) : null}
      </div>
    </div>
  );
}

/** Translation key and interpolation values for a progress phase. */
function phaseKey(progress: RuntimeProgress): [string, Record<string, string>] {
  switch (progress.phase) {
    case 'checking':
      return ['runtime.phaseChecking', {}];
    case 'downloading':
      return ['runtime.phaseDownloading', { version: progress.version }];
    case 'verifying':
      return ['runtime.phaseVerifying', {}];
    case 'installing':
      return ['runtime.phaseInstalling', {}];
    case 'done':
      return ['runtime.phaseDone', {}];
    case 'failed':
      return ['runtime.failed', {}];
  }
}
