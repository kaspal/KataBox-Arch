import { cn } from "@/lib/utils";
import { GLYPHS } from "@/solve/glyphs";

/** A component glyph (decorative: always shown next to its name). */
export function Glyph({ icon, className }: { icon: string; className?: string }) {
  return (
    <svg
      viewBox="0 0 24 24"
      fill="none"
      stroke="currentColor"
      strokeWidth={1.75}
      strokeLinecap="round"
      strokeLinejoin="round"
      aria-hidden
      className={cn("size-4 shrink-0", className)}
      // Static, bundled markup from GLYPHS (never user data).
      dangerouslySetInnerHTML={{ __html: GLYPHS[icon] ?? "" }}
    />
  );
}

/** The category-tinted tile holding a glyph: the colour at 14% on the card. */
export function GlyphTile({ icon, category, className }: { icon: string; category: string; className?: string }) {
  return (
    <span
      className={cn("flex size-7 shrink-0 items-center justify-center rounded-md", className)}
      style={{
        background: `color-mix(in srgb, var(--cat-${category}) 14%, var(--card))`,
        color: `var(--cat-${category}-glyph)`,
      }}
      aria-hidden
    >
      <Glyph icon={icon} />
    </span>
  );
}
