import { useTranslation } from 'react-i18next';

import styles from './BootScreen.module.css';

/** Shown while `app_info` is in flight. Startup should make this a blink. */
export function BootScreen() {
  const { t } = useTranslation();

  return (
    <div className={styles.screen} role="status" aria-live="polite">
      <div className={styles.spinner} aria-hidden="true" />
      <p className={styles.label}>{t('boot.starting')}</p>
    </div>
  );
}

interface BootErrorProps {
  message: string;
  onRetry: () => void;
}

export function BootError({ message, onRetry }: BootErrorProps) {
  const { t } = useTranslation();

  return (
    <div className={styles.screen} role="alert">
      <h1 className={styles.errorTitle}>{t('boot.failedTitle')}</h1>
      <p className={styles.errorMessage}>{message}</p>
      <p className={styles.label}>{t('boot.failedHint')}</p>
      <button type="button" className={styles.retry} onClick={onRetry}>
        {t('common.retry')}
      </button>
    </div>
  );
}
