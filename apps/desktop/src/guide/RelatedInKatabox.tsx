// "Related in KataBox" (M6.7): the KataBox Learn topics and problems that drill the
// algorithm behind this page. Titles only: KataBox is a separate app, and there is no
// deep link into it yet (docs/katabox-crosslinks-proposal.md).
import { useTranslation } from "react-i18next";
import { useQuery } from "@tanstack/react-query";
import { BookOpen, Code2 } from "lucide-react";
import { api } from "@/lib/api";

export function RelatedInKatabox({ target }: { target: string }) {
  const { t, i18n } = useTranslation();
  const es = i18n.language.startsWith("es");
  const links = useQuery({ queryKey: ["katabox-links", target], queryFn: () => api.kataboxLinks(target) });
  if (!links.data || links.data.length === 0) return null;
  return (
    <section aria-labelledby={`katabox-${target}`} className="space-y-2 rounded-lg border bg-card p-4" data-testid="katabox-links">
      <h2 id={`katabox-${target}`} className="text-sm font-semibold">
        {t("katabox.title")}
      </h2>
      <ul className="space-y-1 text-sm">
        {links.data.map((l) => (
          <li key={`${l.kind}:${l.id}`} className="flex items-center gap-2">
            {l.kind === "learn" ? <BookOpen aria-hidden className="size-4 text-muted-foreground" /> : <Code2 aria-hidden className="size-4 text-muted-foreground" />}
            <span>{es ? l.title_es : l.title}</span>
            <span className="text-xs text-muted-foreground">{t(`katabox.kind.${l.kind}`)}</span>
          </li>
        ))}
      </ul>
      <p className="text-xs text-muted-foreground">{t("katabox.hint")}</p>
    </section>
  );
}
