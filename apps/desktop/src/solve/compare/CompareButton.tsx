// "Compare with reference": opens the Compare dialog with the design as it is now. Only shown
// once the problem is solved (the brief carries the reference only then).
import { useState } from "react";
import { useTranslation } from "react-i18next";
import { GitCompareArrows } from "lucide-react";
import type { Design } from "@/bindings";
import { Button } from "@/components/ui/button";
import { useRegistry } from "@/lib/queries";
import { CompareDialog } from "@/solve/compare/CompareDialog";
import { toDesign, useEditor } from "@/solve/store";

export function CompareButton({ reference, onOpen }: { reference: Design; onOpen?: () => void }) {
  const { t } = useTranslation();
  const registry = useRegistry().data;
  const [mine, setMine] = useState<Design | null>(null);
  if (!registry) return null;
  return (
    <>
      <Button
        variant="outline"
        size="sm"
        className="gap-2"
        onClick={() => {
          onOpen?.();
          setMine(toDesign(useEditor.getState()));
        }}
      >
        <GitCompareArrows aria-hidden className="size-4" />
        {t("compare.open")}
      </Button>
      {mine && <CompareDialog open onClose={() => setMine(null)} mine={mine} reference={reference} registry={registry} />}
    </>
  );
}
