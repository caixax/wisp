import { useCallback, useEffect, useRef, useState } from 'react';

import * as ipc from '@/lib/ipc';
import { isAuthFailure, isWispError, type AuthState } from '@/lib/types';

export type AuthPhase = 'checking' | 'signedOut' | 'signedIn';

/**
 * Tracks the Epic account.
 *
 * Checking costs a process spawn, so it happens once after the window is up
 * rather than during boot. The login window reports back on an event, since
 * the user may spend minutes on Epic's page.
 */
export function useAuth(runtimeReady: boolean) {
  const [state, setState] = useState<AuthState | null>(null);
  const [phase, setPhase] = useState<AuthPhase>('checking');
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const apply = useCallback((next: AuthState) => {
    setState(next);
    setPhase(next.loggedIn ? 'signedIn' : 'signedOut');
  }, []);

  useEffect(() => {
    if (!runtimeReady) return;

    let cancelled = false;
    void (async () => {
      try {
        const next = await ipc.authStatus();
        if (!cancelled) apply(next);
      } catch (e) {
        // Legendary could not be asked. Treat it as signed out and let the
        // runtime banner explain why.
        if (cancelled) return;
        setError(isWispError(e) ? e.message : String(e));
        setPhase('signedOut');
      }
    })();

    return () => {
      cancelled = true;
    };
  }, [runtimeReady, apply]);

  // The login window finishes long after the command that opened it, so the
  // result arrives on an event rather than as a return value.
  const applyRef = useRef(apply);
  useEffect(() => {
    applyRef.current = apply;
  }, [apply]);

  useEffect(() => {
    const unlisten = ipc.onAuthChanged((event) => {
      setBusy(false);
      if (isAuthFailure(event)) {
        setError(event.error);
        setPhase('signedOut');
        return;
      }
      setError(null);
      applyRef.current(event);
    });
    return () => {
      void unlisten.then((stop) => stop());
    };
  }, []);

  const login = useCallback(async () => {
    setError(null);
    setBusy(true);
    try {
      await ipc.authLogin();
    } catch (e) {
      setBusy(false);
      setError(isWispError(e) ? e.message : String(e));
    }
  }, []);

  const submitCode = useCallback(
    async (code: string) => {
      setError(null);
      setBusy(true);
      try {
        apply(await ipc.authSubmitCode(code));
      } catch (e) {
        setError(isWispError(e) ? e.message : String(e));
      } finally {
        setBusy(false);
      }
    },
    [apply],
  );

  const logout = useCallback(async () => {
    setBusy(true);
    try {
      apply(await ipc.authLogout());
    } catch (e) {
      setError(isWispError(e) ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  }, [apply]);

  return { state, phase, error, busy, login, submitCode, logout };
}
