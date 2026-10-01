import { useCallback, useState } from 'react';

import * as ipc from '@/lib/ipc';
import { isWispError, type RuntimeProbe, type RuntimeStatus, type UpdateCheck } from '@/lib/types';

/**
 * Owns everything the UI knows about the Legendary runtime, so the settings
 * page and the shell banner read the same state instead of each running their
 * own checks.
 */
export function useRuntime(initial: RuntimeStatus) {
  const [status, setStatus] = useState<RuntimeStatus>(initial);
  const [probe, setProbe] = useState<RuntimeProbe | null>(null);
  const [update, setUpdate] = useState<UpdateCheck | null>(null);
  const [updateError, setUpdateError] = useState<string | null>(null);
  const [checking, setChecking] = useState(false);

  const refreshStatus = useCallback(async () => {
    const next = await ipc.runtimeStatus();
    setStatus(next);
    return next;
  }, []);

  /** Confirms the binary actually runs. Costs about a second. */
  const runProbe = useCallback(async () => {
    const result = await ipc.runtimeProbe();
    setProbe(result);
    return result;
  }, []);

  /**
   * Asks GitHub whether a newer release exists. Failures are kept out of the
   * way: being offline should not produce an error banner on every start.
   */
  const checkUpdate = useCallback(async () => {
    setChecking(true);
    setUpdateError(null);
    try {
      const result = await ipc.runtimeCheckUpdate();
      setUpdate(result);
      return result;
    } catch (error) {
      setUpdateError(isWispError(error) ? error.message : String(error));
      return null;
    } finally {
      setChecking(false);
    }
  }, []);

  /**
   * Called after an install or repair finishes. The previous probe and update
   * results describe a binary that is no longer there, so they are dropped and
   * the new one is verified straight away.
   */
  const applyInstalled = useCallback((next: RuntimeStatus) => {
    setStatus(next);
    setUpdate(null);
    setUpdateError(null);
    setProbe(null);
    void ipc.runtimeProbe().then(setProbe);
  }, []);

  return {
    status,
    probe,
    update,
    updateError,
    checking,
    refreshStatus,
    runProbe,
    checkUpdate,
    applyInstalled,
  };
}
