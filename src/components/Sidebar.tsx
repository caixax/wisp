import type { ReactElement } from 'react';
import { useTranslation } from 'react-i18next';

import type { Route, RouteName } from '@/lib/routes';

import styles from './Sidebar.module.css';

interface SidebarProps {
  current: Route;
  onNavigate: (route: Route) => void;
  version: string;
}

interface NavItem {
  page: Exclude<RouteName, 'game'>;
  labelKey: string;
  /** Outlined icons need stroke instead of fill. */
  outlined?: boolean;
  path: ReactElement;
}

const ITEMS: NavItem[] = [
  {
    page: 'library',
    labelKey: 'nav.library',
    path: (
      <path d="M3 4.5A1.5 1.5 0 0 1 4.5 3h3A1.5 1.5 0 0 1 9 4.5v11A1.5 1.5 0 0 1 7.5 17h-3A1.5 1.5 0 0 1 3 15.5v-11Zm8 0A1.5 1.5 0 0 1 12.5 3h3A1.5 1.5 0 0 1 17 4.5v11a1.5 1.5 0 0 1-1.5 1.5h-3a1.5 1.5 0 0 1-1.5-1.5v-11Z" />
    ),
  },
  {
    page: 'downloads',
    labelKey: 'nav.downloads',
    outlined: true,
    path: (
      <path d="M10 2.75v8.75m0 0 3.5-3.5M10 11.5 6.5 8M3.5 14v1.5A1.5 1.5 0 0 0 5 17h10a1.5 1.5 0 0 0 1.5-1.5V14" />
    ),
  },
  {
    page: 'settings',
    labelKey: 'nav.settings',
    outlined: true,
    path: (
      <>
        <circle cx="10" cy="10" r="2.6" />
        <path d="M16.1 12.1a1.3 1.3 0 0 0 .26 1.43l.05.05a1.56 1.56 0 1 1-2.2 2.2l-.05-.05a1.3 1.3 0 0 0-1.43-.26 1.3 1.3 0 0 0-.79 1.19v.13a1.56 1.56 0 1 1-3.12 0v-.07a1.3 1.3 0 0 0-.85-1.19 1.3 1.3 0 0 0-1.43.26l-.05.05a1.56 1.56 0 1 1-2.2-2.2l.05-.05a1.3 1.3 0 0 0 .26-1.43 1.3 1.3 0 0 0-1.19-.79h-.13a1.56 1.56 0 1 1 0-3.12h.07a1.3 1.3 0 0 0 1.19-.85 1.3 1.3 0 0 0-.26-1.43l-.05-.05a1.56 1.56 0 1 1 2.2-2.2l.05.05a1.3 1.3 0 0 0 1.43.26h.06a1.3 1.3 0 0 0 .79-1.19v-.13a1.56 1.56 0 1 1 3.12 0v.07a1.3 1.3 0 0 0 .79 1.19 1.3 1.3 0 0 0 1.43-.26l.05-.05a1.56 1.56 0 1 1 2.2 2.2l-.05.05a1.3 1.3 0 0 0-.26 1.43v.06a1.3 1.3 0 0 0 1.19.79h.13a1.56 1.56 0 1 1 0 3.12h-.07a1.3 1.3 0 0 0-1.19.79Z" />
      </>
    ),
  },
];

export function Sidebar({ current, onNavigate, version }: SidebarProps) {
  const { t } = useTranslation();

  return (
    <aside className={styles.sidebar}>
      <div className={styles.brand}>
        <WispMark />
        <div>
          <div className={styles.brandName}>{t('app.name')}</div>
          <div className={styles.brandVersion}>{version}</div>
        </div>
      </div>

      <nav className={styles.nav} aria-label={t('nav.sectionsLabel')}>
        {ITEMS.map((item) => {
          const active = current.page === item.page;
          return (
            <button
              key={item.page}
              type="button"
              className={styles.navItem}
              data-active={active}
              aria-current={active ? 'page' : undefined}
              onClick={() => onNavigate({ page: item.page })}
            >
              <svg
                className={styles.icon}
                viewBox="0 0 20 20"
                fill={item.outlined ? 'none' : 'currentColor'}
                stroke={item.outlined ? 'currentColor' : 'none'}
                strokeWidth="1.5"
                strokeLinecap="round"
                strokeLinejoin="round"
                aria-hidden="true"
              >
                {item.path}
              </svg>
              {t(item.labelKey)}
            </button>
          );
        })}
      </nav>

      <p className={styles.tagline}>{t('app.tagline')}</p>
    </aside>
  );
}

function WispMark() {
  return (
    <svg className={styles.mark} viewBox="0 0 32 32" aria-hidden="true">
      <defs>
        <linearGradient id="wisp-mark" x1="0" y1="1" x2="0.3" y2="0">
          <stop offset="0%" stopColor="#7c5eff" />
          <stop offset="55%" stopColor="#56d8ff" />
          <stop offset="100%" stopColor="#ecfdff" />
        </linearGradient>
      </defs>
      <path
        d="M16 3c.6 5.4 5.5 8.4 5.5 14a5.5 5.5 0 0 1-11 0c0-3.3 2.2-5.3 3.4-8.2.7 1.7 1.6 2.6 2.1 4C16.9 9.4 15.6 6.4 16 3Z"
        fill="url(#wisp-mark)"
      />
    </svg>
  );
}
