// The tutorial's dev handle, for the e2e lesson spec: `window.__jackiohTutorial`, set only outside a
// production build, exactly like `window.__jackiohPractice` (routes/practice.tsx). The spec performs
// `suggested` through the real UI, so the coach's own intent drives the browser test; the page never reads it.

import { useEffect } from "react";

import type { ActionBody } from "@jackioh/shared";

import type { CoachDisplay } from "./coach.ts";
import { suggestedAction, type CoachTracker } from "./tracker.ts";

const DEV_ONLY = import.meta.env.MODE !== "production";

export type TutorialDevHandle = {
  readonly lessonId: string;
  readonly display: CoachDisplay;
  readonly suggested: ActionBody | null;
};

declare global {
  interface Window {
    /** Set while a lesson is running. */
    __jackiohTutorial?: TutorialDevHandle;
  }
}

export function useTutorialDevHandle(tracker: CoachTracker | null, lessonId: string | null): void {
  useEffect(() => {
    if (!DEV_ONLY || tracker === null || lessonId === null) return;
    const handle: TutorialDevHandle = {
      lessonId,
      get display() {
        return tracker.getState().display;
      },
      get suggested() {
        return suggestedAction(tracker.script, tracker.getState());
      },
    };
    window.__jackiohTutorial = handle;
    return () => {
      if (window.__jackiohTutorial === handle) delete window.__jackiohTutorial;
    };
  }, [tracker, lessonId]);
}
