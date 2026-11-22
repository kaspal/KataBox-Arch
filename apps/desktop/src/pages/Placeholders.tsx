// Pages that later milestones fill in. Each says what arrives and when.
import { useTranslation } from "react-i18next";
import { BarChart3 } from "lucide-react";
import { EmptyState, Page } from "@/components/Page";

export function Progress() {
  const { t } = useTranslation();
  return (
    <Page title={t("nav.progress")}>
      <EmptyState icon={BarChart3} title={t("placeholder.progress.title")} body={t("placeholder.progress.body")} />
    </Page>
  );
}
