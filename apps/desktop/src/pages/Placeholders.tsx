// Pages that later milestones fill in. Each says what arrives and when.
import { useTranslation } from "react-i18next";
import { BarChart3, BookOpen, List, PlayCircle } from "lucide-react";
import { EmptyState, Page } from "@/components/Page";

export function Problems() {
  const { t } = useTranslation();
  return (
    <Page title={t("nav.problems")}>
      <EmptyState icon={List} title={t("placeholder.problems.title")} body={t("placeholder.problems.body")} />
    </Page>
  );
}

export function Guide() {
  const { t } = useTranslation();
  return (
    <Page title={t("nav.guide")}>
      <EmptyState icon={BookOpen} title={t("placeholder.guide.title")} body={t("placeholder.guide.body")} />
    </Page>
  );
}

export function Samples() {
  const { t } = useTranslation();
  return (
    <Page title={t("nav.samples")}>
      <EmptyState icon={PlayCircle} title={t("placeholder.samples.title")} body={t("placeholder.samples.body")} />
    </Page>
  );
}

export function Progress() {
  const { t } = useTranslation();
  return (
    <Page title={t("nav.progress")}>
      <EmptyState icon={BarChart3} title={t("placeholder.progress.title")} body={t("placeholder.progress.body")} />
    </Page>
  );
}
