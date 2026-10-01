import { useTranslation } from 'react-i18next';

import type { RuntimeProbe, UpdateCheck } from '@/lib/types';

import { Button } from './Button';

import styles from './RuntimeBanner.module.css';

interface RuntimeBannerProps {
  probe: RuntimeProbe | null;
  update: UpdateCheck | null;
  busy: boolean;
  onRepair: () => void;
  onUpdate: () => void;
  onDismissUpdate: () => void;
}

/**
 * Surfaces the two runtime conditions the user has to know about: a binary
 * that does not run, and a new Legendary release. Updates are offered, never
 * applied on their own.
 */
export function RuntimeBanner({
  probe,
  update,
  busy,
  onRepair,
  onUpdate,
  onDismissUpdate,
}: RuntimeBannerProps) {
  const { t } = useTranslation();

  if (probe && !probe.ok) {
    return (
      <div className={styles.banner} data-tone="danger" role="alert">
        <div className={styles.text}>
          <strong>{t('runtime.brokenTitle')}</strong>
          <span>{probe.error ?? t('runtime.brokenBody')}</span>
        </div>
        <Button variant="primary" onClick={onRepair} disabled={busy}>
          {t('runtime.repair')}
        </Button>
      </div>
    );
  }

  if (update?.updateAvailable) {
    return (
      <div className={styles.banner} data-tone="info">
        <div className={styles.text}>
          <strong>{t('runtime.updateAvailable', { version: update.latest })}</strong>
          <span>{t('runtime.updateHint')}</span>
        </div>
        <Button variant="primary" onClick={onUpdate} disabled={busy}>
          {t('runtime.update')}
        </Button>
        <Button variant="ghost" onClick={onDismissUpdate} aria-label={t('common.close')}>
          ✕
        </Button>
      </div>
    );
  }

  return null;
}
