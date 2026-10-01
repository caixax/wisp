import { useTranslation } from 'react-i18next';

import { EmptyState, Page } from '@/components/Page';

export function DownloadsPage() {
  const { t } = useTranslation();

  return (
    <Page title={t('downloads.title')} subtitle={t('downloads.subtitle')}>
      <EmptyState title={t('downloads.emptyTitle')} body={t('downloads.emptyBody')} />
    </Page>
  );
}
