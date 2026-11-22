// /estimate: the estimation trainer (M5.2). Pick a problem's drills, or a random one; every
// answer is checked in Rust and comes back with its worked solution.
import { useMemo, useState } from "react";
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useSearch } from "@tanstack/react-router";
import { Calculator, Shuffle } from "lucide-react";
import { EmptyState, Page } from "@/components/Page";
import { Button } from "@/components/ui/button";
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from "@/components/ui/select";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useDrills, useEstimateStatuses, useProblems } from "@/lib/queries";
import { useLocalized } from "@/pages/Problems";
import { EstimateDrill } from "@/practice/EstimateDrill";

export function EstimatePage() {
  const { t, i18n } = useTranslation();
  const loc = useLocalized();
  const { problem } = useSearch({ from: "/estimate" });
  const navigate = useNavigate();
  const drills = useDrills();
  const problems = useProblems();
  const statuses = useEstimateStatuses();
  const [pick, setPick] = useState<string | null>(null);

  const withDrills = useMemo(() => {
    const ids = new Set((drills.data ?? []).map((d) => d.problem_id));
    return (problems.data ?? []).filter((p) => ids.has(p.id));
  }, [drills.data, problems.data]);
  const done = new Set((statuses.data ?? []).filter((s) => s.done).map((s) => `${s.problem_id}/${s.drill_id}`));

  if (drills.error || problems.error) {
    const e = drills.error ?? problems.error;
    return (
      <Page title={t("estimate.title")}>
        <EmptyState icon={Calculator} title={t("guide.unavailable")} body={describeError(i18n, e).summary} />
      </Page>
    );
  }
  if (!drills.data || !problems.data) {
    return (
      <Page title={t("estimate.title")}>
        <Skeleton className="h-80 w-full" />
      </Page>
    );
  }
  const current = problem ?? withDrills[0]?.id;
  const shown = pick
    ? drills.data.filter((d) => `${d.problem_id}/${d.id}` === pick)
    : drills.data.filter((d) => d.problem_id === current);
  const random = () => {
    const pool = drills.data.filter((d) => !done.has(`${d.problem_id}/${d.id}`));
    const from = pool.length > 0 ? pool : drills.data;
    // A user's pick, not a simulation: Math.random is fine here.
    const d = from[Math.floor(Math.random() * from.length)];
    if (d) setPick(`${d.problem_id}/${d.id}`);
  };
  const total = drills.data.length;
  const doneCount = drills.data.filter((d) => done.has(`${d.problem_id}/${d.id}`)).length;

  return (
    <Page
      title={t("estimate.title")}
      actions={
        <div className="flex items-center gap-2">
          <Select
            value={pick ? "" : current}
            onValueChange={(id) => {
              setPick(null);
              void navigate({ to: "/estimate", search: { problem: id } });
            }}
          >
            <SelectTrigger className="w-64 bg-card" aria-label={t("estimate.problem")}>
              <SelectValue placeholder={t("estimate.problem")} />
            </SelectTrigger>
            <SelectContent>
              {withDrills.map((p) => (
                <SelectItem key={p.id} value={p.id}>
                  #{p.number} {loc.title(p)}
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
          <Button variant="outline" className="gap-2" onClick={random}>
            <Shuffle aria-hidden className="size-4" />
            {t("estimate.random")}
          </Button>
        </div>
      }
    >
      <p className="mb-6 max-w-2xl text-sm text-muted-foreground">
        {t("estimate.intro")}{" "}
        <Link to="/numbers" className="font-semibold text-primary underline-offset-4 hover:underline">
          {t("numbers.title")}
        </Link>
      </p>
      <p className="mb-4 text-sm" data-testid="estimate-progress">
        {t("estimate.progress", { done: doneCount, total })}
      </p>
      <div className="max-w-3xl space-y-4">
        {shown.map((d) => (
          <EstimateDrill key={`${d.problem_id}/${d.id}`} drill={d} done={done.has(`${d.problem_id}/${d.id}`)} />
        ))}
      </div>
    </Page>
  );
}
