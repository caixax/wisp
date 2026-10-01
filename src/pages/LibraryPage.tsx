import { useTranslation } from 'react-i18next';

import { EmptyState, Page } from '@/components/Page';

export function LibraryPage() {
  const { t } = useTranslation();

  return (
    <Page title={t('library.title')} subtitle={t('library.subtitle')}>
      <EmptyState title={t('library.emptyTitle')} body={t('library.emptyBody')} />
    </Page>
  );
}
