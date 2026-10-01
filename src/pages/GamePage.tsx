import { useEffect, useState } from 'react';
import { useTranslation } from 'react-i18next';
import { convertFileSrc } from '@tauri-apps/api/core';

import { Button } from '@/components/Button';
import { EmptyState, Page } from '@/components/Page';
import { formatBytes } from '@/lib/format';
import * as ipc from '@/lib/ipc';
import { isWispError, type GameDetails } from '@/lib/types';

import styles from './GamePage.module.css';

interface GamePageProps {
  appName: string;
  coverVersion: number;
  onBack: () => void;
}

type State =
  | { kind: 'loading' }
  | { kind: 'ready'; game: GameDetails }
  | { kind: 'missing' }
  | { kind: 'failed'; message: string };

export function GamePage({ appName, coverVersion, onBack }: GamePageProps) {
  const { t, i18n } = useTranslation();
  const [state, setState] = useState<State>({ kind: 'loading' });

  useEffect(() => {
    let cancelled = false;
    void (async () => {
      try {
        const game = await ipc.gameDetails(appName);
        if (cancelled) return;
        setState(game ? { kind: 'ready', game } : { kind: 'missing' });
      } catch (e) {
        if (!cancelled) {
          setState({ kind: 'failed', message: isWispError(e) ? e.message : String(e) });
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  }, [appName]);

  const back = (
    <Button variant="ghost" onClick={onBack}>
      {t('game.back')}
    </Button>
  );

  if (state.kind === 'loading') {
    return (
      <Page title={t('common.loading')} actions={back}>
        <EmptyState title={t('common.loading')} body={t('game.loadingBody')} />
      </Page>
    );
  }

  if (state.kind === 'missing') {
    return (
      <Page title={t('game.notFoundTitle')} actions={back}>
        <EmptyState title={t('game.notFoundTitle')} body={t('game.notFoundBody')} />
      </Page>
    );
  }

  if (state.kind === 'failed') {
    return (
      <Page title={t('game.notFoundTitle')} actions={back}>
        <EmptyState title={t('game.notFoundTitle')} body={state.message} />
      </Page>
    );
  }

  const { game } = state;
  const installed = game.installed;

  return (
    <Page title={game.title} subtitle={game.developer ?? undefined} actions={back}>
      <div className={styles.header}>
        <img
          className={styles.cover}
          src={`${convertFileSrc(game.appName, 'cover')}?v=${coverVersion}`}
          alt=""
          onError={(event) => {
            event.currentTarget.style.visibility = 'hidden';
          }}
        />

        <div className={styles.facts}>
          <dl className={styles.list}>
            <Fact
              label={t('game.status')}
              value={installed ? t('game.statusInstalled') : t('game.statusNotInstalled')}
            />
            {installed ? (
              <Fact
                label={t('game.size')}
                value={formatBytes(installed.installSize, i18n.language)}
              />
            ) : null}
            <Fact
              label={t('game.installedVersion')}
              value={installed?.version ?? t('game.none')}
              mono={Boolean(installed)}
            />
            <Fact
              label={t('game.availableVersion')}
              value={game.availableVersion ?? t('game.none')}
              mono={Boolean(game.availableVersion)}
            />
            {installed ? (
              <Fact label={t('game.location')} value={installed.installPath} mono />
            ) : null}
            <Fact
              label={t('game.cloudSaves')}
              value={game.cloudSaves ? t('game.supported') : t('game.notSupported')}
            />
          </dl>

          <div className={styles.flags}>
            {game.updateAvailable ? (
              <span className={styles.flag} data-tone="accent">
                {t('library.badgeUpdate')}
              </span>
            ) : null}
            {installed?.needsVerification ? (
              <span className={styles.flag} data-tone="warn">
                {t('game.needsVerification')}
              </span>
            ) : null}
            {installed?.canRunOffline ? (
              <span className={styles.flag}>{t('game.offlineCapable')}</span>
            ) : null}
          </div>

          {game.thirdPartyStore ? (
            <p className={styles.warning}>
              {t('game.thirdParty', { store: game.thirdPartyStore })}
            </p>
          ) : null}
          {!game.installable && !game.thirdPartyStore ? (
            <p className={styles.warning}>{t('game.noBuild')}</p>
          ) : null}

          <p className={styles.soon}>{t('game.actionsSoon')}</p>
        </div>
      </div>

      {game.description ? (
        <section className={styles.section}>
          <h2 className={styles.sectionTitle}>{t('game.about')}</h2>
          <p className={styles.description}>{game.description}</p>
        </section>
      ) : null}

      <section className={styles.section}>
        <h2 className={styles.sectionTitle}>
          {t('game.dlc')} <span className={styles.counter}>{game.dlcs.length}</span>
        </h2>

        {game.dlcs.length === 0 ? (
          <p className={styles.none}>{t('game.noDlc')}</p>
        ) : (
          <ul className={styles.dlcList}>
            {game.dlcs.map((dlc) => (
              <li key={dlc.appName} className={styles.dlc}>
                <span className={styles.dlcTitle}>{dlc.title}</span>
                <span className={styles.dlcState}>
                  {dlc.installed
                    ? dlc.updateAvailable
                      ? t('game.dlcUpdate')
                      : t('game.dlcInstalled')
                    : t('game.dlcNotInstalled')}
                </span>
              </li>
            ))}
          </ul>
        )}
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
