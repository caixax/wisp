import { useEffect } from 'react';

import type { Theme } from '@/lib/types';

const DARK_QUERY = '(prefers-color-scheme: dark)';

/**
 * Applies the chosen theme to `<html data-theme>`.
 *
 * `system` follows the OS preference live, so switching the desktop theme
 * recolours Wisp without a restart.
 */
export function useTheme(theme: Theme): void {
  useEffect(() => {
    const root = document.documentElement;

    if (theme !== 'system') {
      root.dataset.theme = theme;
      return;
    }

    const media = window.matchMedia(DARK_QUERY);
    const apply = () => {
      root.dataset.theme = media.matches ? 'dark' : 'light';
    };

    apply();
    media.addEventListener('change', apply);
    return () => media.removeEventListener('change', apply);
  }, [theme]);
}
