// The life strip and the teammate strip (#936 B61, B63), on the Duos board's rail. Both read the
// `DuosView` and nothing else (CLAUDE.md rule 7): life is public, so both teams' own HP and pools are
// in it (B23), and the teammate's mana, hand and whether they have ended are the team's own.
//
// THE LIFE STRIP. One row per team, the seat's own first: each seat's own HP (`duos-own-hp`, the
// seat on `data-seat`) and the team's pool (`duos-pool`, the team on `data-team`). A seat whose own
// HP is spent (B13) no longer has a bar of its own: it reads "On the pool", and the pool, which now
// takes its hits, is marked as carrying it (`data-carrying`). Within a row the seat in this board's
// duel comes first, so the first `duos-own-hp` is always the viewer's own.
//
// THE TEAMMATE STRIP (`duos-teammate-strip`): their HP, mana, hand count and whether they have ended
// their turn this round, always, whichever board the portal shows; and, from the route, whether
// their socket is up and the AI is playing for them meanwhile (B37).

import type { ReactElement } from "react";

import { DUOS_OWN_HEALTH } from "@jackioh/engine/config";

import type { DuosSeat } from "../wire/generated/DuosSeat.ts";
import type { DuosSideLife } from "../wire/generated/DuosSideLife.ts";
import type { DuosTeamView } from "../wire/generated/DuosTeamView.ts";
import type { DuosView } from "../wire/generated/DuosView.ts";
import { PERCENT } from "../game/clockConstants.ts";
import { t, useLocale } from "../i18n/i18n.ts";
import { acrossOf, duelOf, seatLabel, teammateOf } from "./seats.ts";
import { duosTestid } from "./testids.ts";

/** A bar's fill, as a share of `cap`, kept inside the bar. */
function fill(value: number, cap: number): string {
  const share = cap <= 0 ? 0 : (value / cap) * PERCENT;
  return `${String(Math.max(0, Math.min(PERCENT, share)))}%`;
}

function SeatLife({ seat, life, you }: { seat: DuosSeat; life: DuosSideLife | undefined; you: boolean }): ReactElement | null {
  if (life === undefined) return null;
  const who = you ? t("You") : seatLabel(seat);
  return (
    <span
      className="duos-life__own"
      data-testid={duosTestid.ownHp}
      data-seat={seat}
      data-value={life.own}
      data-spent={life.spent ? "true" : "false"}
      data-you={you ? "true" : undefined}
      role="img"
      aria-label={
        life.spent
          ? t("{who}: own health spent, hits come off the pool", { who })
          : t("{who}: {health} health", { who, health: life.own })
      }
    >
      <span className="duos-life__who" aria-hidden="true">
        {who}
      </span>
      {life.spent ? (
        <span className="duos-life__spent" aria-hidden="true">
          {t("On the pool")}
        </span>
      ) : (
        <>
          <span className="duos-life__bar" aria-hidden="true">
            <span className="duos-life__fill" style={{ width: fill(life.own, DUOS_OWN_HEALTH) }} />
          </span>
          <span className="duos-life__value" aria-hidden="true">
            {life.own}
          </span>
        </>
      )}
    </span>
  );
}

function TeamLife({
  label,
  team,
  seats,
  you,
  own,
}: {
  label: string;
  team: DuosTeamView;
  /** The team's two seats, the one in the viewer's duel first. */
  seats: readonly [DuosSeat, DuosSeat];
  you: DuosSeat;
  own: boolean;
}): ReactElement {
  const carrying = team.sides.some((side) => side.spent);
  return (
    <div className="duos-life__team" data-team={team.team} data-side={own ? "team" : "enemy"} role="group" aria-label={label}>
      <span className="duos-life__label" aria-hidden="true">
        {label}
      </span>
      {seats.map((seat) => (
        <SeatLife key={seat} seat={seat} life={team.sides[duelOf(seat)]} you={seat === you} />
      ))}
      <span
        className="duos-life__pool"
        data-testid={duosTestid.pool}
        data-team={team.team}
        data-value={team.pool}
        data-cap={team.poolCap}
        data-carrying={carrying ? "true" : "false"}
        role="img"
        aria-label={t("Pool: {pool} of {cap}", { pool: team.pool, cap: team.poolCap })}
      >
        <span className="duos-life__who" aria-hidden="true">
          {t("Pool")}
        </span>
        <span className="duos-life__bar duos-life__bar--pool" aria-hidden="true">
          <span className="duos-life__fill" style={{ width: fill(team.pool, team.poolCap) }} />
        </span>
        <span className="duos-life__value" aria-hidden="true">
          {team.pool}/{team.poolCap}
        </span>
      </span>
    </div>
  );
}

/** B61: both teams' own HP and pools. */
export function LifeStrip({ view }: { view: DuosView }): ReactElement {
  useLocale();
  const you = view.seat;
  const mate = teammateOf(you);
  return (
    <div className="duos-life" data-testid={duosTestid.lifeStrip}>
      <TeamLife label={t("Your team")} team={view.team} seats={[you, mate]} you={you} own />
      <TeamLife label={t("Enemy team")} team={view.enemy} seats={[acrossOf(you), acrossOf(mate)]} you={you} own={false} />
    </div>
  );
}

export type TeammateLink = { connected: boolean; aiPlaying: boolean } | null;

/** B63: the teammate at a glance, whichever board is showing. */
export function TeammateStrip({ view, link }: { view: DuosView; link: TeammateLink }): ReactElement {
  useLocale();
  const seat = teammateOf(view.seat);
  const slot = duelOf(seat);
  const life = view.team.sides[slot];
  const ended = view.team.ended[slot] === true;
  const side = view.teammate.you;
  const hand = Array.isArray(side.hand) ? side.hand.length : side.hand.count;
  const theirTurn = view.result === null && view.teammate.active === view.teammate.viewer;
  return (
    <div
      className="duos-teammate"
      data-testid={duosTestid.teammateStrip}
      data-seat={seat}
      data-ended={ended ? "true" : "false"}
      data-hand={hand}
      data-mana={side.mana.current}
      data-connected={link === null ? undefined : link.connected ? "true" : "false"}
      data-ai-playing={link?.aiPlaying === true ? "true" : undefined}
    >
      <span className="duos-teammate__name">
        {t("Teammate")} · {seatLabel(seat)}
      </span>{" "}
      {/* The spaces between the strip's parts are for its text (a screen reader's, a copy's); its flex
          layout draws none of them. */}
      <span className="duos-teammate__hp">
        {life === undefined ? null : life.spent ? t("On the pool") : t("{health} health", { health: life.own })}
      </span>{" "}
      <span className="duos-teammate__mana">
        {t("{current}/{max} mana", { current: side.mana.current, max: side.mana.max })}
      </span>{" "}
      <span className="duos-teammate__hand">
        {hand === 1 ? t("1 card in hand") : t("{count} cards in hand", { count: hand })}
      </span>{" "}
      <span className="duos-teammate__turn" data-state={ended ? "ended" : theirTurn ? "playing" : "waiting"}>
        {ended ? t("Ended their turn") : theirTurn ? t("Playing") : t("Waiting")}
      </span>
      {link !== null && !link.connected ? (
        <span className="duos-teammate__away" role="status">
          {link.aiPlaying ? t("Disconnected: the AI is playing for them") : t("Disconnected")}
        </span>
      ) : null}
    </div>
  );
}
