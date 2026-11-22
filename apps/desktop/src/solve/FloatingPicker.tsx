// A small keyboard-first list that opens at a point on the canvas: the flavour picker on
// drop, and the protocol picker when a wire is made. Arrow keys move, Enter or a click
// picks, Escape or a click outside cancels.
import { useEffect, useRef, useState } from "react";
import { cn } from "@/lib/utils";

export type PickerOption<T extends string> = { value: T; label: string; hint?: string };

export function FloatingPicker<T extends string>({
  title,
  options,
  initial,
  at,
  onPick,
  onCancel,
}: {
  title: string;
  options: PickerOption<T>[];
  initial?: T;
  /** Position inside the canvas wrapper, in pixels. */
  at: { x: number; y: number };
  onPick: (value: T) => void;
  onCancel: () => void;
}) {
  const [active, setActive] = useState(Math.max(0, options.findIndex((o) => o.value === initial)));
  const list = useRef<HTMLDivElement>(null);
  const id = useRef(`picker-${Math.random().toString(36).slice(2)}`).current;

  useEffect(() => {
    list.current?.focus();
  }, []);

  useEffect(() => {
    const onDown = (e: PointerEvent) => {
      if (list.current && !list.current.contains(e.target as globalThis.Node)) onCancel();
    };
    window.addEventListener("pointerdown", onDown, true);
    return () => window.removeEventListener("pointerdown", onDown, true);
  }, [onCancel]);

  return (
    <div
      className="absolute z-20 min-w-48 rounded-lg border bg-popover p-1 text-popover-foreground shadow-lg"
      style={{ left: at.x, top: at.y }}
      data-testid="floating-picker"
    >
      <div id={`${id}-title`} className="px-2 py-1 text-xs font-medium text-muted-foreground">
        {title}
      </div>
      <div
        ref={list}
        role="listbox"
        tabIndex={0}
        aria-labelledby={`${id}-title`}
        aria-activedescendant={`${id}-${active}`}
        className="outline-none"
        onKeyDown={(e) => {
          e.stopPropagation();
          if (e.key === "ArrowDown") {
            e.preventDefault();
            setActive((i) => (i + 1) % options.length);
          } else if (e.key === "ArrowUp") {
            e.preventDefault();
            setActive((i) => (i - 1 + options.length) % options.length);
          } else if (e.key === "Enter") {
            e.preventDefault();
            onPick(options[active].value);
          } else if (e.key === "Escape") {
            e.preventDefault();
            onCancel();
          }
        }}
      >
        {options.map((o, i) => (
          <div
            key={o.value}
            id={`${id}-${i}`}
            role="option"
            aria-selected={i === active}
            className={cn(
              "flex cursor-default items-center justify-between gap-4 rounded-md px-2 py-1.5 text-sm",
              // A ring as well as the fill: accent is close to the popover in dark mode.
              i === active && "bg-accent ring-1 ring-primary ring-inset",
            )}
            onPointerEnter={() => setActive(i)}
            onClick={() => onPick(o.value)}
          >
            <span>{o.label}</span>
            {o.hint && <span className="text-xs text-muted-foreground">{o.hint}</span>}
          </div>
        ))}
      </div>
    </div>
  );
}
