import { useState } from 'react';
import { useTranslation } from 'react-i18next';
import { convertFileSrc } from '@tauri-apps/api/core';

import type { GameSummary } from '@/lib/types';

import styles from './GameCard.module.css';

interface GameCardProps {
  game: GameSummary;
  /** Changes when new cover art lands, so placeholders retry their image. */
  coverVersion: number;
  onOpen: (appName: string) => void;
}

export function GameCard({ game, coverVersion, onOpen }: GameCardProps) {
  const { t } = useTranslation();
  const [failed, setFailed] = useState(false);

  // The backend serves cached covers on its own protocol. The version is a
  // cache buster: without it the webview keeps the 404 it saw first.
  const source = `${convertFileSrc(game.appName, 'cover')}?v=${coverVersion}`;

  return (
    <button type="button" className={styles.card} onClick={() => onOpen(game.appName)}>
      <div className={styles.art}>
        {failed ? (
          <span className={styles.placeholder} aria-hidden="true">
            {initials(game.title)}
          </span>
        ) : (
          <img
            key={coverVersion}
            className={styles.image}
            src={source}
            alt=""
            loading="lazy"
            decoding="async"
            onError={() => setFailed(true)}
          />
        )}

        <div className={styles.badges}>
          {game.updateAvailable ? (
            <span className={styles.badge} data-tone="accent">
              {t('library.badgeUpdate')}
            </span>
          ) : null}
          {game.installed ? (
            <span className={styles.badge} data-tone="installed">
              {t('library.badgeInstalled')}
            </span>
          ) : null}
          {!game.installable ? (
            <span className={styles.badge} data-tone="muted">
              {t('library.badgeUnavailable')}
            </span>
          ) : null}
        </div>
      </div>

      <span className={styles.title}>{game.title}</span>
      {game.developer ? <span className={styles.developer}>{game.developer}</span> : null}
    </button>
  );
}

/** Up to two letters, for titles with no cover art. */
function initials(title: string): string {
  return title
    .split(/\s+/)
    .filter(Boolean)
    .slice(0, 2)
    .map((word) => word[0]?.toUpperCase() ?? '')
    .join('');
}
