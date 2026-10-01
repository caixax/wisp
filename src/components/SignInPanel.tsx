import { useState } from 'react';
import { useTranslation } from 'react-i18next';

import type { useAuth } from '@/hooks/useAuth';
import * as ipc from '@/lib/ipc';

import { Button } from './Button';

import styles from './SignInPanel.module.css';

interface SignInPanelProps {
  auth: ReturnType<typeof useAuth>;
}

/**
 * Sign-in prompt.
 *
 * The normal path opens Epic's login page in its own window and picks the
 * code up automatically. The manual path exists because embedded browsers do
 * fail: it opens the same page in the real browser and takes the code by
 * hand, which is what Legendary asks for when its webview is unavailable.
 */
export function SignInPanel({ auth }: SignInPanelProps) {
  const { t } = useTranslation();
  const [manual, setManual] = useState(false);
  const [code, setCode] = useState('');

  const submit = () => {
    const trimmed = code.trim();
    if (trimmed) void auth.submitCode(trimmed);
  };

  return (
    <div className={styles.panel}>
      <h2 className={styles.title}>{t('auth.title')}</h2>
      <p className={styles.body}>{t('auth.body')}</p>
      <p className={styles.note}>{t('auth.privacy')}</p>

      <div className={styles.actions}>
        <Button variant="primary" onClick={() => void auth.login()} disabled={auth.busy}>
          {auth.busy ? t('auth.waiting') : t('auth.signIn')}
        </Button>
        <Button variant="ghost" onClick={() => setManual((open) => !open)}>
          {manual ? t('auth.hideManual') : t('auth.useManual')}
        </Button>
      </div>

      {manual ? (
        <div className={styles.manual}>
          <ol className={styles.steps}>
            <li>
              {t('auth.manualStep1')}{' '}
              <button type="button" className={styles.link} onClick={() => void openLoginPage()}>
                {t('auth.manualOpen')}
              </button>
            </li>
            <li>{t('auth.manualStep2')}</li>
            <li>{t('auth.manualStep3')}</li>
          </ol>

          <div className={styles.codeRow}>
            <input
              type="text"
              className={styles.code}
              placeholder={t('auth.codePlaceholder')}
              aria-label={t('auth.codePlaceholder')}
              value={code}
              spellCheck={false}
              autoComplete="off"
              onChange={(event) => setCode(event.target.value)}
              onKeyDown={(event) => {
                if (event.key === 'Enter') submit();
              }}
            />
            <Button
              variant="primary"
              onClick={submit}
              disabled={auth.busy || code.trim().length === 0}
            >
              {t('auth.submitCode')}
            </Button>
          </div>
        </div>
      ) : null}

      {auth.error ? (
        <p className={styles.error} role="alert">
          {auth.error}
        </p>
      ) : null}
    </div>
  );
}

async function openLoginPage() {
  const url = await ipc.authLoginUrl();
  const { openUrl } = await import('@tauri-apps/plugin-opener');
  await openUrl(url);
}
