// Quiet UI ticks (SPEC §10.11): a click on anything pressable, and a hover blip when a mouse moves onto
// a new pressable element, delegated from one root so no component knows about sound. Hover is mouse
// only: a touch or pen `pointerover` fires right before the tap's click, two sounds for one touch.

import { UI_HOVER_THROTTLE_MS } from "./constants.ts";
import type { SoundSink } from "./types.ts";

export const UI_CLICK_SELECTOR = 'button:not(:disabled), [role="button"]:not([aria-disabled="true"]), [data-legal="true"]';
export const UI_HOVER_SELECTOR = 'button:not(:disabled), [data-legal="true"]';

function elementOf(target: EventTarget | null): Element | null {
  return target instanceof Element ? target : null;
}

export function installUiSounds(
  sink: Pick<SoundSink, "playSfx">,
  host: Document | HTMLElement,
  now: () => number = () => performance.now(),
): () => void {
  let lastHovered: Element | null = null;
  let lastHoverAt: number | null = null;

  const onClick = (event: Event): void => {
    const element = elementOf(event.target);
    if (element === null) return;
    if (element.closest(UI_CLICK_SELECTOR) === null) return;
    sink.playSfx("uiClick");
  };

  const onPointerOver = (event: Event): void => {
    if ((event as Partial<PointerEvent>).pointerType !== "mouse") return;
    const element = elementOf(event.target);
    if (element === null) return;
    const hit = element.closest(UI_HOVER_SELECTOR);
    if (hit === null || hit === lastHovered) return;
    lastHovered = hit;
    const at = now();
    if (lastHoverAt !== null && at - lastHoverAt < UI_HOVER_THROTTLE_MS) return;
    lastHoverAt = at;
    sink.playSfx("uiHover");
  };

  const listen = {
    // Capture phase: a component that stops propagation still gets its tick.
    capture: true,
    // Stryker disable next-line BooleanLiteral: neither listener calls preventDefault, so a passive listener and an active one behave alike
    passive: true,
  };
  host.addEventListener("click", onClick, listen);
  host.addEventListener("pointerover", onPointerOver, listen);

  return () => {
    host.removeEventListener("click", onClick, listen);
    host.removeEventListener("pointerover", onPointerOver, listen);
  };
}
