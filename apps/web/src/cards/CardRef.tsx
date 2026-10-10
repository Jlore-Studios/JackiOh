// Card-text references (SPEC §10.10, R279) open their printed, public catalog face in interactive views.
// They never reveal card state (§5.1; CLAUDE.md rule 7).
// Escape closes only the tooltip; the detail view and sheet retain their own handling (inspect/store.ts).

import { useEffect, useId, useLayoutEffect, useRef, useState, type ReactElement, type ReactNode } from "react";
import { createPortal } from "react-dom";

import type { CardDef } from "@jackioh/shared";

import { CardFace } from "./CardFace.tsx";
import { FACE_ASPECT, REF_HOVER_DELAY_MS, REF_TOOLTIP_HEIGHT_PX } from "./constants.ts";
import { placePreview, type Rect } from "./inspect/placement.ts";
import { faceModel } from "./model.ts";
import { RefsInteractive, useRefsInteractive } from "./refContext.tsx";

export type CardRefProps = { def: CardDef; radiant: boolean; children: ReactNode };

export const REF_TOOLTIP_TESTID = "card-ref-tooltip";

type Timer = ReturnType<typeof setTimeout>;

type Press = { touch: boolean; wasOpen: boolean };

function rectOf(element: Element): Rect {
  const box = element.getBoundingClientRect();
  return { left: box.left, top: box.top, width: box.width, height: box.height };
}

function RefTooltip({ id, def, radiant, anchor }: { id: string; def: CardDef; radiant: boolean; anchor: Rect }): ReactElement {
  const height = Math.min(REF_TOOLTIP_HEIGHT_PX, window.innerHeight);
  const size = { width: height * FACE_ASPECT, height };
  const placed = placePreview(anchor, { width: window.innerWidth, height: window.innerHeight }, size, "above");
  const face = faceModel({ defId: def.id, def, radiant });
  return createPortal(
    <div
      id={id}
      role="tooltip"
      className="cf-ref-tooltip"
      data-testid={REF_TOOLTIP_TESTID}
      data-ref={def.id}
      data-ref-face={radiant ? "radiant" : "base"}
      style={{ position: "fixed", left: placed.left, top: placed.top, width: size.width, height: size.height }}
    >
      {/* The face in a tooltip marks its own references and opens none: one level is enough. */}
      <RefsInteractive enabled={false}>
        <CardFace face={face} layout="full" />
      </RefsInteractive>
    </div>,
    document.body,
  );
}

export function CardRef({ def, radiant, children }: CardRefProps): ReactElement {
  const interactive = useRefsInteractive();
  const tooltipId = useId();
  const ref = useRef<HTMLSpanElement>(null);
  const timer = useRef<Timer | null>(null);
  const press = useRef<Press | null>(null);
  const [anchor, setAnchor] = useState<Rect | null>(null);

  const clearTimer = (): void => {
    if (timer.current !== null) clearTimeout(timer.current);
    timer.current = null;
  };
  const open = (): void => {
    const element = ref.current;
    if (element !== null && element.isConnected) setAnchor(rectOf(element));
  };
  const close = (): void => {
    clearTimer();
    setAnchor(null);
  };

  useEffect(() => clearTimer, []);

  // Scroll moves an open tooltip: focusing a reference may scroll its dialog without closing it.
  const isOpen = anchor !== null;
  useLayoutEffect(() => {
    if (!isOpen) return undefined;
    const onKey = (event: KeyboardEvent): void => {
      if (event.key === "Escape") close();
    };
    const onPressElsewhere = (event: PointerEvent): void => {
      const element = ref.current;
      if (element !== null && event.target instanceof Node && element.contains(event.target)) return;
      close();
    };
    const onMove = (): void => {
      const element = ref.current;
      if (element !== null && element.isConnected) setAnchor(rectOf(element));
      else close();
    };
    window.addEventListener("keydown", onKey, true);
    window.addEventListener("pointerdown", onPressElsewhere, true);
    window.addEventListener("scroll", onMove, true);
    window.addEventListener("resize", onMove);
    return () => {
      window.removeEventListener("keydown", onKey, true);
      window.removeEventListener("pointerdown", onPressElsewhere, true);
      window.removeEventListener("scroll", onMove, true);
      window.removeEventListener("resize", onMove);
    };
  }, [isOpen]);

  const common = {
    ref,
    className: "cf-ref",
    "data-ref": def.id,
    "data-ref-face": radiant ? "radiant" : "base",
  } as const;

  if (!interactive) return <span {...common}>{children}</span>;

  return (
    <>
      <span
        {...common}
        data-ref-open={anchor === null ? undefined : "true"}
        tabIndex={0}
        aria-describedby={anchor === null ? undefined : tooltipId}
        onPointerEnter={(event) => {
          if (event.pointerType === "touch") return;
          clearTimer();
          timer.current = setTimeout(open, REF_HOVER_DELAY_MS);
        }}
        onPointerLeave={(event) => {
          if (event.pointerType === "touch") return;
          close();
        }}
        onPointerDown={(event) => {
          // Record state before focus opens it so a touch tap can close an already-open tooltip.
          press.current = { touch: event.pointerType === "touch", wasOpen: anchor !== null };
        }}
        onFocus={open}
        onBlur={close}
        onClick={(event) => {
          // Stop the card underneath receiving a pick; a touch tap toggles.
          event.stopPropagation();
          const pressed = press.current;
          press.current = null;
          if (pressed !== null && pressed.touch && pressed.wasOpen) close();
          else open();
        }}
        onKeyDown={(event) => {
          if (event.key !== "Enter" && event.key !== " ") return;
          event.preventDefault();
          if (anchor === null) open();
          else close();
        }}
      >
        {children}
      </span>
      {anchor === null ? null : <RefTooltip id={tooltipId} def={def} radiant={radiant} anchor={anchor} />}
    </>
  );
}
