// The left navigation (matches the Home mockup). Settings sits at the bottom of the rail.
import { BarChart3, BookOpen, Home, LayoutGrid, List, PlayCircle, Settings, type LucideIcon } from "lucide-react";

export type NavItem = {
  to: "/" | "/problems" | "/sandbox" | "/guide" | "/samples" | "/progress" | "/settings";
  labelKey:
    | "nav.home"
    | "nav.problems"
    | "nav.sandbox"
    | "nav.guide"
    | "nav.samples"
    | "nav.progress"
    | "nav.settings";
  icon: LucideIcon;
};

export const NAV: NavItem[] = [
  { to: "/", labelKey: "nav.home", icon: Home },
  { to: "/problems", labelKey: "nav.problems", icon: List },
  { to: "/sandbox", labelKey: "nav.sandbox", icon: LayoutGrid },
  { to: "/guide", labelKey: "nav.guide", icon: BookOpen },
  { to: "/samples", labelKey: "nav.samples", icon: PlayCircle },
  { to: "/progress", labelKey: "nav.progress", icon: BarChart3 },
];

export const SETTINGS_NAV: NavItem = { to: "/settings", labelKey: "nav.settings", icon: Settings };
