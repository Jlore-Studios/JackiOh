// #2877: the hero's status tray: its modifiers, secrets and Rift flag in one row hung off the
// plate, out of flow so the plate keeps one size however many effects the seat carries.
//
// The badges read the `SideView` alone and decide nothing (CLAUDE.md rule 7). R169's list stays
// the `modifiers-<side>` element, always in the DOM so `modifierChanged` has an element to fade
// even when its last badge is gone (BUILD M5-T4); R860's secrets keep `secrets-<side>` and R847's
// flag keeps `rift-spent-<side>`, each with the chip markup Hero.tsx drew. Past the layout's
// allowance (`heroLayout.ts`) the last slot is a `+N` chip opening every entry in a bubble placed
// and dismissed the way `ModifierBadge.tsx`'s bubble is; each modifier in it is still a
// `ModifierBadge`, so #2321's full label opens per entry there too.

import { useEffect, useId, useRef, useState, type CSSProperties, type ReactElement } from "react";
import { createPortal } from "react-dom";

import type { ModifierView, SideView } from "@jackioh/shared";

/** One secret of a seat, as `SideView.secrets` carries it (R860). */
type SecretView = NonNullable<SideView["secrets"]>[number];

import { t } from "../i18n/i18n.ts";
import { animTestid } from "./animations.ts";
import { bubblePlacement, ModifierBadge } from "./ModifierBadge.tsx";
import { type AnimatingMap, type Side } from "./contract.ts";
import { useHeroTrayAllowance } from "./heroLayout.ts";
import { secretBadgeText } from "./secrets.ts";

/** R847: the badge a side wears once a Temporal Rift has granted it its one extra turn, and its tooltip. */
export const RIFT_SPENT_LABEL = "Rift turn used";
export const RIFT_SPENT_NOTE = "Temporal Rift has granted this player's one extra turn this game; another Rift grants none";

type TrayEntry =
  | { kind: "modifier"; modifier: ModifierView }
  | { kind: "secret"; secret: SecretView }
  | { kind: "rift" };

function SecretChip({ secret }: { secret: SecretView }): ReactElement {
  return (
    <span className="modifier-badge secret-badge" data-secret-id={secret.id} data-choice={secret.choice}>
      {secretBadgeText(secret.choice)}
    </span>
  );
}

function RiftChip(): ReactElement {
  return (
    <span className="modifier-badge rift-badge" data-rift-extra-turn="true" title={t(RIFT_SPENT_NOTE)}>
      {t(RIFT_SPENT_LABEL)}
    </span>
  );
}

export default function HeroTray({
  side,
  seat,
  animating,
}: {
  side: Side;
  seat: SideView;
  animating?: AnimatingMap;
}): ReactElement {
  const allowance = useHeroTrayAllowance();
  const modifiers = seat.modifiers ?? [];
  const secrets = seat.secrets ?? [];

  const entries: TrayEntry[] = [
    ...modifiers.map((modifier): TrayEntry => ({ kind: "modifier", modifier })),
    ...secrets.map((secret): TrayEntry => ({ kind: "secret", secret })),
    ...(seat.riftExtraTurn === true ? [{ kind: "rift" } as TrayEntry] : []),
  ];
  const total = entries.length;
  const overflow = total > allowance;
  // The `+N` chip takes the allowance's last slot, so one fewer entry renders as a chip.
  const visible = overflow ? entries.slice(0, allowance - 1) : entries;
  const hidden = overflow ? total - (allowance - 1) : 0;
  const visibleModifiers = visible.filter((entry) => entry.kind === "modifier");
  const visibleSecrets = visible.filter((entry) => entry.kind === "secret");
  const visibleRift = visible.some((entry) => entry.kind === "rift");

  const [at, setAt] = useState<CSSProperties | null>(null);
  const listId = useId();
  const more = useRef<HTMLButtonElement>(null);
  const list = useRef<HTMLSpanElement>(null);
  const open = at !== null;

  useEffect(() => {
    if (!open) return;
    function close(): void {
      setAt(null);
    }
    function onPointer(event: PointerEvent): void {
      if (!(event.target instanceof Node)) return;
      if (more.current?.contains(event.target) === true || list.current?.contains(event.target) === true) return;
      close();
    }
    window.addEventListener("pointerdown", onPointer);
    // The list is placed once; a scroll or a resize would leave it behind, so it goes.
    window.addEventListener("resize", close);
    window.addEventListener("scroll", close, true);
    return () => {
      window.removeEventListener("pointerdown", onPointer);
      window.removeEventListener("resize", close);
      window.removeEventListener("scroll", close, true);
    };
  }, [open ]);

  const modifiersId = animTestid.modifiers(side);

  return (
    <span className="hero-tray" data-testid={`tray-${side}`} data-count={total}>
      {/* R169: one badge per `SideView.modifiers` entry, in the view's order. Always present, so
          `modifierChanged` has an element to fade even when the badge that changed is the one that
          has just gone. */}
      <span
        className="modifiers"
        data-testid={modifiersId}
        data-count={modifiers.length}
        data-animating={animating?.get(modifiersId)}
        // WCAG 4.1.2 (#757): a name needs a role.
        role="group"
        aria-label={side === "you" ? t("Your modifiers") : t("Opponent modifiers")}
      >
        {/* #2321: each opens its whole label on a tap, which a touch screen's missing hover cannot. */}
        {visibleModifiers.map((entry) =>
          entry.kind === "modifier" ? <ModifierBadge key={entry.modifier.id} modifier={entry.modifier} /> : null,
        )}
      </span>

      {/* R860: one badge per secret this seat holds, in the view's order. The badge names the
          choice only where the view carries it — the owner's seat, or anyone once revealed —
          and the opponent's badge says only "Secret". */}
      {secrets.length > 0 && (
        <span className="modifiers secrets" data-testid={`secrets-${side}`}>
          {visibleSecrets.map((entry) =>
            entry.kind === "secret" ? <SecretChip key={entry.secret.id} secret={entry.secret} /> : null,
          )}
        </span>
      )}

      {/* R847: the once-a-game Temporal Rift flag, public on both seats. Outside `modifiers-<side>`,
          whose badges are the view's modifiers (R169). */}
      {seat.riftExtraTurn === true && (
        <span className="modifiers rift" data-testid={`rift-spent-${side}`}>
          {visibleRift ? <RiftChip /> : null}
        </span>
      )}

      {overflow && (
        <button
          type="button"
          ref={more}
          className="modifier-badge tray-more"
          data-testid={`tray-more-${side}`}
          data-more={String(hidden)}
          title={t("{count} more effects", { count: hidden })}
          aria-expanded={open}
          aria-controls={open ? listId : undefined}
          onClick={(event) => {
            event.stopPropagation();
            const rect = more.current?.getBoundingClientRect();
            setAt(open || rect === undefined ? null : bubblePlacement(rect));
          }}
          // Focus leaving for anything but the list (Tab away) closes it, as a badge's bubble does.
          onBlur={(event) => {
            if (open && !(event.relatedTarget instanceof Node && list.current?.contains(event.relatedTarget) === true))
              setAt(null);
          }}
        >
          {`+${hidden}`}
        </button>
      )}
      {at === null
        ? null
        : createPortal(
            // tabIndex -1: a click on the list focuses it, so reading it is no blur that closes it.
            <span
              ref={list}
              id={listId}
              className="hero-tray-list modifier-full"
              data-testid={`tray-list-${side}`}
              role="note"
              tabIndex={-1}
              style={at}
            >
              {entries.map((entry) =>
                entry.kind === "modifier" ? (
                  <ModifierBadge key={entry.modifier.id} modifier={entry.modifier} />
                ) : entry.kind === "secret" ? (
                  <SecretChip key={entry.secret.id} secret={entry.secret} />
                ) : (
                  <RiftChip key="rift" />
                ),
              )}
            </span>,
            document.body,
          )}
    </span>
  );
}
