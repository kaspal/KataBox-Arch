// /problems/$id: the Sandbox layout with a problem. The draft autosaves; Submit judges the
// design on every test (Rust) and opens the results; "Replay at failure" plays that test from
// just before its bottleneck appeared, with the card selected.
import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { useQueryClient } from "@tanstack/react-query";
import { useParams } from "@tanstack/react-router";
import { List } from "lucide-react";
import type { Design, SubmitResult, TestResult } from "@/bindings";
import { useReportError } from "@/app/dialogs";
import { EmptyState, Page } from "@/components/Page";
import { Skeleton } from "@/components/ui/skeleton";
import { api } from "@/lib/api";
import { localDay } from "@/lib/dates";
import { describeError } from "@/lib/errors";
import { queryKeys, useProblemBrief } from "@/lib/queries";
import { ProblemContext } from "@/solve/problem/context";
import { ResultsDialog } from "@/solve/problem/ResultsDialog";
import { useRun } from "@/solve/run/runStore";
import { SandboxPage } from "@/solve/SandboxPage";
import { docJson, newId, SCHEMA_VERSION, toDesign, useEditor } from "@/solve/store";

/** Wait this long after the last edit before saving the draft. */
const DRAFT_DELAY_MS = 800;

function freshDesign(problemId: string, name: string): Design {
  return {
    schema_version: SCHEMA_VERSION,
    id: newId("design"),
    name,
    problem_id: problemId,
    nodes: [],
    edges: [],
    groups: [],
    notes: [],
    viewport: { x: 0, y: 0, zoom: 1 },
  };
}

export function ProblemPage() {
  const { t, i18n } = useTranslation();
  const { id } = useParams({ from: "/problems/$id" });
  const brief = useProblemBrief(id);
  const qc = useQueryClient();
  const report = useReportError();
  const [submitting, setSubmitting] = useState(false);
  const [result, setResult] = useState<SubmitResult | null>(null);
  const loaded = useRef<string | null>(null);
  const es = i18n.language.startsWith("es");

  // Load the editor once per problem: a design for this problem already open (from a file, or
  // left from earlier) stays unless it is empty; otherwise the saved draft (for a Find-the-flaw
  // drill, its broken design), or an empty design for it.
  useEffect(() => {
    const b = brief.data;
    if (!b || loaded.current === id) return;
    loaded.current = id;
    const open = useEditor.getState().doc;
    if (open.problem_id !== id || (open.nodes.length === 0 && (b.draft?.nodes.length ?? 0) > 0)) {
      useEditor.getState().load(b.draft ?? freshDesign(id, es ? b.summary.title_es : b.summary.title_en), null);
      // H6: a draft (a replay's design as it was, a flaw drill's broken design) opens in view.
      if (b.draft) useEditor.getState().requestFit();
    }
    const first = b.tests.find((x) => !x.hidden) ?? b.tests[0];
    useRun.getState().setProblemTest(first ? { problem_id: id, test_id: first.id } : null);
  }, [brief.data, id, es]);
  useEffect(() => () => useRun.getState().setProblemTest(null), []);

  // Autosave the draft; a design not saved to a file is then "clean" (no close prompt).
  useEffect(() => {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const unsub = useEditor.subscribe((s, prev) => {
      if (s.doc === prev.doc || loaded.current !== id) return;
      clearTimeout(timer);
      timer = setTimeout(() => {
        const st = useEditor.getState();
        api
          .saveDraft(id, toDesign(st))
          .then(() => {
            const now = useEditor.getState();
            if (now.path === null && now.doc === st.doc) useEditor.setState({ savedJson: docJson(st.doc) });
          })
          .catch(() => {});
      }, DRAFT_DELAY_MS);
    });
    return () => {
      clearTimeout(timer);
      unsub();
    };
  }, [id]);

  const submit = async () => {
    setSubmitting(true);
    try {
      const r = await api.submitDesign(id, toDesign(useEditor.getState()), localDay());
      setResult(r);
      await Promise.all([
        qc.invalidateQueries({ queryKey: queryKeys.brief(id) }),
        qc.invalidateQueries({ queryKey: queryKeys.problems }),
        qc.invalidateQueries({ queryKey: queryKeys.progress }),
        qc.invalidateQueries({ queryKey: queryKeys.submissions(id) }),
      ]);
    } catch (e) {
      report(e);
    } finally {
      setSubmitting(false);
    }
  };

  const replay = (test: TestResult) => {
    setResult(null);
    const run = useRun.getState();
    run.setProblemTest({ problem_id: id, test_id: test.id });
    if (test.bottleneck) useEditor.getState().setSelection([test.bottleneck.node]);
    run.start(toDesign(useEditor.getState()), { startS: test.replay_at_s ?? 0 }).catch(report);
  };

  if (brief.error) {
    return (
      <Page title={t("nav.problems")}>
        <EmptyState icon={List} title={t("problems.unavailable")} body={describeError(i18n, brief.error).summary} />
      </Page>
    );
  }
  if (!brief.data) {
    return (
      <div className="flex flex-1 gap-4 p-4">
        <Skeleton className="h-full w-64" />
        <Skeleton className="h-full flex-1" />
      </div>
    );
  }
  return (
    <ProblemContext.Provider value={{ id, brief: brief.data, submitting, submit: () => void submit() }}>
      <SandboxPage />
      <ResultsDialog result={result} onClose={() => setResult(null)} onReplay={replay} />
    </ProblemContext.Provider>
  );
}
