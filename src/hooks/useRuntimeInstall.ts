import { useCallback, useEffect, useRef, useState } from 'react';
import type { UnlistenFn } from '@tauri-apps/api/event';

import * as ipc from '@/lib/ipc';
import { isWispError, type RuntimeProgress, type RuntimeStatus } from '@/lib/types';

export type InstallState =
  | { kind: 'idle' }
  | { kind: 'running'; progress: RuntimeProgress }
  | { kind: 'done'; status: RuntimeStatus }
  | { kind: 'failed'; message: string };

/**
 * Drives a runtime install or repair and exposes its progress.
 *
 * Shared by first-run setup, the "update Legendary" action and the "repair
 * runtime" button, because all three are the same download with a different
 * label. The progress subscription is set up before the install starts, so no
 * early event is missed.
 */
export function useRuntimeInstall(onInstalled?: (status: RuntimeStatus) => void) {
  const [state, setState] = useState<InstallState>({ kind: 'idle' });

  // Kept in a ref so callers do not have to memoise their callback.
  const installed = useRef(onInstalled);
  useEffect(() => {
    installed.current = onInstalled;
  }, [onInstalled]);

  const run = useCallback(async (force = false) => {
    setState({ kind: 'running', progress: { phase: 'checking' } });

    let unlisten: UnlistenFn | undefined;
    try {
      unlisten = await ipc.onRuntimeProgress((progress) => {
        // The terminal phases are taken from the command's own result, so a
        // failed install reports the error rather than a bare event payload.
        if (progress.phase !== 'done' && progress.phase !== 'failed') {
          setState({ kind: 'running', progress });
        }
      });

      const result = await ipc.runtimeInstall(force);
      setState({ kind: 'done', status: result });
      installed.current?.(result);
      return result;
    } catch (error) {
      setState({
        kind: 'failed',
        message: isWispError(error) ? error.message : String(error),
      });
      return null;
    } finally {
      unlisten?.();
    }
  }, []);

  const reset = useCallback(() => setState({ kind: 'idle' }), []);

  return { state, run, reset };
}
