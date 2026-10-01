import { useMemo, useState } from 'react';
import { useTranslation } from 'react-i18next';

import { Button } from '@/components/Button';
import { GameCard } from '@/components/GameCard';
import { EmptyState, Page } from '@/components/Page';
import { SignInPanel } from '@/components/SignInPanel';
import type { useAuth } from '@/hooks/useAuth';
import type { useLibrary } from '@/hooks/useLibrary';
import { formatBytes } from '@/lib/format';
import type { GameSummary } from '@/lib/types';

import styles from './LibraryPage.module.css';

type Filter = 'all' | 'installed' | 'updates';
type Sort = 'title' | 'size';

const FILTERS: Filter[] = ['all', 'installed', 'updates'];

interface LibraryPageProps {
  auth: ReturnType<typeof useAuth>;
  library: ReturnType<typeof useLibrary>;
  onOpenGame: (appName: string) => void;
}

export function LibraryPage({ auth, library, onOpenGame }: LibraryPageProps) {
  const { t, i18n } = useTranslation();
  const [query, setQuery] = useState('');
  const [filter, setFilter] = useState<Filter>('all');
  const [sort, setSort] = useState<Sort>('title');

  const { snapshot, loading, refreshing, error, refresh, coverVersion, covers, coversPending } =
    library;

  const visible = useMemo(
    () => sortGames(filterGames(snapshot.games, filter, query), sort),
    [snapshot.games, filter, query, sort],
  );

  if (auth.phase === 'checking') {
    return (
      <Page title={t('library.title')} subtitle={t('library.subtitle')}>
        <EmptyState title={t('library.checkingAccount')} body={t('library.checkingAccountBody')} />
      </Page>
    );
  }

  if (auth.phase === 'signedOut') {
    return (
      <Page title={t('library.title')} subtitle={t('library.subtitle')}>
        <SignInPanel auth={auth} />
      </Page>
    );
  }

  const counts = {
    all: snapshot.games.length,
    installed: snapshot.installedCount,
    updates: snapshot.updateCount,
  };

  return (
    <Page
      title={t('library.title')}
      subtitle={t('library.countSubtitle', { count: snapshot.games.length })}
      actions={
        <Button onClick={() => void refresh(false)} disabled={refreshing}>
          {refreshing ? t('library.refreshing') : t('library.refresh')}
        </Button>
      }
    >
      <div className={styles.toolbar}>
        <input
          type="search"
          className={styles.search}
          placeholder={t('library.searchPlaceholder')}
          aria-label={t('library.searchPlaceholder')}
          value={query}
          onChange={(event) => setQuery(event.target.value)}
        />

        <div className={styles.filters} role="group" aria-label={t('library.filterLabel')}>
          {FILTERS.map((option) => (
            <button
              key={option}
              type="button"
              className={styles.filter}
              data-active={filter === option}
              aria-pressed={filter === option}
              onClick={() => setFilter(option)}
            >
              {t(`library.filter_${option}`)}
              <span className={styles.count}>{counts[option]}</span>
            </button>
          ))}
        </div>

        <label className={styles.sort}>
          <span className={styles.sortLabel}>{t('library.sortLabel')}</span>
          <select
            className={styles.select}
            value={sort}
            onChange={(event) => setSort(event.target.value as Sort)}
          >
            <option value="title">{t('library.sortTitle')}</option>
            <option value="size">{t('library.sortSize')}</option>
          </select>
        </label>
      </div>

      {error ? (
        <p className={styles.error} role="alert">
          {error}
        </p>
      ) : null}

      {coversPending && covers ? (
        <p className={styles.notice}>
          {t('library.downloadingCovers', { done: covers.done, total: covers.total })}
        </p>
      ) : null}

      {visible.length === 0 ? (
        <EmptyState
          title={t(...emptyTitle(loading || refreshing, snapshot.games.length))}
          body={t(...emptyBody(loading || refreshing, snapshot.games.length, query))}
        />
      ) : (
        <>
          <div className={styles.grid}>
            {visible.map((game) => (
              <GameCard
                key={game.appName}
                game={game}
                coverVersion={coverVersion}
                onOpen={onOpenGame}
              />
            ))}
          </div>
          {sort === 'size' ? (
            <p className={styles.footnote}>
              {t('library.sizeFootnote', {
                size: formatBytes(installedBytes(snapshot.games), i18n.language),
              })}
            </p>
          ) : null}
        </>
      )}
    </Page>
  );
}

function filterGames(games: GameSummary[], filter: Filter, query: string): GameSummary[] {
  const needle = query.trim().toLowerCase();

  return games.filter((game) => {
    if (filter === 'installed' && !game.installed) return false;
    if (filter === 'updates' && !game.updateAvailable) return false;
    if (!needle) return true;
    return (
      game.title.toLowerCase().includes(needle) ||
      (game.developer?.toLowerCase().includes(needle) ?? false)
    );
  });
}

/** Size sorting puts the biggest installs first; titles with no install last. */
function sortGames(games: GameSummary[], sort: Sort): GameSummary[] {
  if (sort === 'title') return games;

  return [...games].sort((a, b) => {
    const sizeA = a.installed?.installSize ?? -1;
    const sizeB = b.installed?.installSize ?? -1;
    if (sizeA === sizeB) return a.title.localeCompare(b.title);
    return sizeB - sizeA;
  });
}

function installedBytes(games: GameSummary[]): number {
  return games.reduce((total, game) => total + (game.installed?.installSize ?? 0), 0);
}

/** Translation key and values, so the caller owns the `t` call. */
type Phrase = [key: string, values: Record<string, string>];

function emptyTitle(busy: boolean, total: number): Phrase {
  if (busy) return ['library.loading', {}];
  if (total === 0) return ['library.emptyTitle', {}];
  return ['library.noMatchesTitle', {}];
}

function emptyBody(busy: boolean, total: number, query: string): Phrase {
  if (busy) return ['library.loadingBody', {}];
  if (total === 0) return ['library.emptyBody', {}];
  return query ? ['library.noMatchesBody', { query }] : ['library.noMatchesFilterBody', {}];
}
