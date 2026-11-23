// #2877: the hero's status tray: its modifiers, secrets and Rift flag in one row hung off the
// plate, out of flow so the plate keeps one size however many effects the seat carries.
// #4251: the hero's counters (Luck, Burn, Toxic, Tipsy) ride the same row as labelled chips,
// outside the overflow allowance, so every counter reads at once.
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

import { Icon } from "../cards/index.ts";
import { t } from "../i18n/i18n.ts";
import { animTestid } from "./animations.ts";
import { bubblePlacement, ModifierBadge } from "./ModifierBadge.tsx";
import { type AnimatingMap, type Side } from "./contract.ts";
import { useHeroTrayAllowance } from "./heroLayout.ts";
import { secretBadgeText } from "./secrets.ts";

/** R847: the badge a side wears once a Temporal Rift has granted it its one extra turn, and its tooltip. */
export const RIFT_SPENT_LABEL = "Rift turn used";
export const RIFT_SPENT_NOTE = "Temporal Rift has granted this player's one extra turn this game; another Rift grants none";

type CounterKind = "luck" | "burn" | "toxic" | "tipsy";

/**
 * #4251: one of the hero's counters (R987's Luck, R3540000's Burn and Toxic, R3548000's Tipsy)
 * as a labelled chip in the tray, its name as text beside its count, so a touch screen reads
 * which counter it is with no hover. The class, `data-*` attribute and (for Luck) testid are the
 * portrait badges' own, only their parent moved, so no counter hides under the health, Armor or
 * Credits badges; each still draws only above 0.
 */
function CounterChip({ kind, value }: { kind: CounterKind; value: number }): ReactElement {
  const name = kind === "luck" ? t("Luck") : kind === "burn" ? t("Burn") : kind === "toxic" ? t("Toxic") : t("Tipsy");
  const title =
    kind === "luck"
      ? t("Luck {luck}: every roll your cards make that keeps a best rolls {luck} more times", { luck: String(value) })
      : kind === "burn"
        ? t("Burn counters")
        : kind === "toxic"
          ? t("Toxic counters")
          : t("Tipsy counter");
  return (
    <span
      className={`modifier-badge hero-counter hero-${kind}`}
      data-luck={kind === "luck" ? value : undefined}
      data-burn={kind === "burn" ? value : undefined}
      data-toxic={kind === "toxic" ? value : undefined}
      data-tipsy={kind === "tipsy" ? value : undefined}
      data-testid={kind === "luck" ? "hero-luck" : undefined}
      title={title}
      aria-label={`${name} ${String(value)}`}
    >
      {kind === "tipsy" ? <Icon name="stein" /> : null}
      <span className="hero-counter-name">{name}</span> <span className="hero-counter-n">{value}</span>
    </span>
  );
}

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

  // #4251: the hero's counters above 0, outside the overflow allowance: they are never folded
  // into the `+N` list, so every counter reads at once however many effects the seat carries.
  const counters: { kind: CounterKind; value: number }[] = [];
  if ((seat.luck ?? 0) > 0) counters.push({ kind: "luck", value: seat.luck ?? 0 });
  if ((seat.hero.burn ?? 0) > 0) counters.push({ kind: "burn", value: seat.hero.burn ?? 0 });
  if ((seat.hero.toxic ?? 0) > 0) counters.push({ kind: "toxic", value: seat.hero.toxic ?? 0 });
  if ((seat.tipsy ?? 0) > 0) counters.push({ kind: "tipsy", value: seat.tipsy ?? 0 });

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

      {/* #4251: the hero's counters above 0, beside the modifiers and outside the `+N`
          allowance, so gaining one never moves the plate and none ever folds away. */}
      {counters.length > 0 && (
        <span className="hero-counters" data-testid={`hero-counters-${side}`}>
          {counters.map((counter) => (
            <CounterChip key={counter.kind} kind={counter.kind} value={counter.value} />
          ))}
        </span>
      )}

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
