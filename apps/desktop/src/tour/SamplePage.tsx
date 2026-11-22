// /samples/$id: one sample in the walkthrough player. "Try it yourself" copies the step's
// design (new ids, no problem) into the Sandbox.
import { useTranslation } from "react-i18next";
import { Link, useNavigate, useParams } from "@tanstack/react-router";
import { ChevronLeft, PlayCircle } from "lucide-react";
import type { Design } from "@/bindings";
import { EmptyState, Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { describeError } from "@/lib/errors";
import { useRegistry, useSample } from "@/lib/queries";
import { useEditor } from "@/solve/store";
import { forkDesign } from "@/tour/fork";
import { Player } from "@/tour/Player";

export function SamplePage() {
  const { t, i18n } = useTranslation();
  const { id } = useParams({ from: "/samples/$id" });
  const sample = useSample(id);
  const registry = useRegistry();
  const navigate = useNavigate();
  const es = i18n.language.startsWith("es");

  if (sample.error) {
    return (
      <Page title={t("nav.samples")}>
        <EmptyState icon={PlayCircle} title={t("samples.unavailable")} body={describeError(i18n, sample.error).summary} />
      </Page>
    );
  }
  if (!sample.data || !registry.data) {
    return <Skeleton className="m-4 h-[calc(100%-2rem)]" />;
  }
  const def = sample.data.def;
  const title = es ? def.title_es : def.title;
  const tryIt = (d: Design) => {
    useEditor.getState().load(forkDesign(d, t("samples.copyName", { title })), null);
    void navigate({ to: "/sandbox" });
  };
  return (
    <div className="flex h-full min-h-0 flex-col">
      <Player
        story={{ title, problemId: def.problem ?? null, steps: def.steps, snapshots: sample.data.snapshots }}
        registry={registry.data}
        onTry={tryIt}
        header={
          <>
            <Link to="/samples" className="flex items-center gap-1 text-sm text-muted-foreground hover:text-foreground">
              <ChevronLeft aria-hidden className="size-4" />
              {t("nav.samples")}
            </Link>
            <span aria-hidden className="text-muted-foreground">
              /
            </span>
            <h1 className="truncate font-display text-2xl font-semibold">{title}</h1>
          </>
        }
      />
    </div>
  );
}
