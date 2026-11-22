// A sticky note in the hand-written face. Double-click (or Enter) to edit; resizable.
import { memo, useEffect, useRef, useState } from "react";
import { NodeResizer, type Node as FlowNode, type NodeProps } from "@xyflow/react";
import { useTranslation } from "react-i18next";
import type { Note } from "@/bindings";
import { cn } from "@/lib/utils";
import { useEditor } from "@/solve/store";

export type NoteData = { note: Note; maxLength: number };
export type NoteFlowNode = FlowNode<NoteData, "note">;

export const NoteNode = memo(function NoteNode({ data, selected }: NodeProps<NoteFlowNode>) {
  const { t } = useTranslation();
  const { note, maxLength } = data;
  const updateNote = useEditor((s) => s.updateNote);
  const beginDrag = useEditor((s) => s.beginDrag);
  const endDrag = useEditor((s) => s.endDrag);
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(note.text);
  const area = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    if (!editing) setDraft(note.text);
  }, [note.text, editing]);
  useEffect(() => {
    if (editing) area.current?.focus();
  }, [editing]);

  const commit = () => {
    setEditing(false);
    if (draft !== note.text) updateNote(note.id, { text: draft });
  };

  return (
    <>
      <NodeResizer
        isVisible={selected && !editing}
        minWidth={120}
        minHeight={60}
        // Live sizes arrive through the canvas's onNodesChange; one resize is one undo step.
        onResizeStart={() => beginDrag()}
        onResizeEnd={() => endDrag()}
      />
      <div
        className={cn(
          "h-full w-full rounded-sm border bg-note p-3 font-hand text-base leading-snug text-note-foreground shadow-[0_1px_0_var(--note-border)]",
          selected ? "border-primary" : "border-[var(--note-border)]",
        )}
        onDoubleClick={() => setEditing(true)}
        onKeyDown={(e) => {
          if (!editing && e.key === "Enter") {
            e.preventDefault();
            setEditing(true);
          }
        }}
        data-testid="note"
      >
        {editing ? (
          <textarea
            ref={area}
            className="nodrag nowheel h-full w-full resize-none bg-transparent font-hand text-base outline-none"
            value={draft}
            maxLength={maxLength}
            aria-label={t("canvas.noteText")}
            onChange={(e) => setDraft(e.target.value)}
            onBlur={commit}
            onKeyDown={(e) => {
              e.stopPropagation();
              if (e.key === "Escape") {
                setDraft(note.text);
                setEditing(false);
              }
            }}
          />
        ) : (
          <p className="h-full overflow-hidden whitespace-pre-wrap break-words">
            {note.text || <span className="text-muted-foreground">{t("canvas.notePlaceholder")}</span>}
          </p>
        )}
      </div>
    </>
  );
});
