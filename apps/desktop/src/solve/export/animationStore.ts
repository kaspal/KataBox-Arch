// Which animation the "Save an animation" dialog is showing (M7b.2). Kept apart from the
// dialog so non-React code and the test setup can open or reset it without importing the API.
import { create } from "zustand";
import type { FramesSource } from "@/lib/api";

export type AnimationRequest = {
  source: FramesSource;
  /** What the run is, for the dialog and the file name ("Sandbox · Normal traffic"). */
  label: string;
  /** The earliest start (Replay's, for a submission) and the test's length, in seconds. */
  minStart: number;
  durationS: number;
};

export const useAnimation = create<{ request: AnimationRequest | null }>(() => ({ request: null }));

export function openAnimation(request: AnimationRequest) {
  useAnimation.setState({ request });
}

export function resetAnimation() {
  useAnimation.setState({ request: null });
}
