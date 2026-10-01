import { useCallback, useEffect, useState } from 'react';

import * as ipc from '@/lib/ipc';
import { isWispError, type CoverProgress, type LibrarySnapshot } from '@/lib/types';

const EMPTY: LibrarySnapshot = {
  games: [],
  installedCount: 0,
  updateCount: 0,
  updatedAt: null,
  fetched: false,
};

/**
 * The game library.
 *
 * The cached snapshot loads instantly, so the grid is on screen before
 * Legendary has been asked anything. A refresh replaces it. Cover art lands
 * afterwards in the background, and `coverVersion` changes so tiles that were
 * showing a placeholder reload their image.
 */
export function useLibrary(signedIn: boolean) {
  const [snapshot, setSnapshot] = useState<LibrarySnapshot>(EMPTY);
  const [loading, setLoading] = useState(true);
  const [refreshing, setRefreshing] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [covers, setCovers] = useState<CoverProgress | null>(null);
  const [coverVersion, setCoverVersion] = useState(0);

  const refresh = useCallback(async (force = false) => {
    setRefreshing(true);
    setError(null);
    try {
      setSnapshot(await ipc.libraryRefresh(force));
      return true;
    } catch (e) {
      setError(isWispError(e) ? e.message : String(e));
      return false;
    } finally {
      setRefreshing(false);
    }
  }, []);

  // Load the cache, then fetch for real if this library has never been read.
  useEffect(() => {
    if (!signedIn) return;

    let cancelled = false;
    void (async () => {
      try {
        const cached = await ipc.librarySnapshot();
        if (cancelled) return;
        setSnapshot(cached);
        if (!cached.fetched) await refresh(false);
      } catch (e) {
        if (!cancelled) setError(isWispError(e) ? e.message : String(e));
      } finally {
        if (!cancelled) setLoading(false);
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [signedIn, refresh]);

  useEffect(() => {
    const unlisten = ipc.onCoverProgress((progress) => {
      setCovers(progress);
      setCoverVersion((version) => version + 1);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const coversPending = covers !== null && covers.done < covers.total;

  return {
    // Signing out leaves the last library in state; never show it to the
    // next account. The effect refills it on the way back in.
    snapshot: signedIn ? snapshot : EMPTY,
    loading: signedIn && loading,
    refreshing,
    error,
    refresh,
    coverVersion,
    covers,
    coversPending,
  };
}
