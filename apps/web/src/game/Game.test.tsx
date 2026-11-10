// Game.tsx's integration-QA findings: the moves of a view the board is not showing yet, the
// game-over state (Result.tsx), and animations that played twice.

import { CATALOG } from "@jackioh/cards";
import type { ActionBody, GameEvent, PlayerView } from "@jackioh/shared";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { StrictMode, useState, type ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { ANIMATIONS, HIDDEN_ID } from "./animations.ts";
import { DAMAGE_FEEL, SLAM_LAND_AT, UNIT_SLAM } from "./damageFeel.ts";
import { CatalogContext, lookupFromDefs } from "./catalog.ts";
import { testid } from "./contract.ts";
import Game, { GAME_HOST_ATTRIBUTE } from "./Game.tsx";
import { resultReason } from "./Result.tsx";
import { __resetSettingsForTests, writeSettings } from "../settings/store.ts";
import { baseView, card, emptySide, faceDownBackrow, pendingFor, unit, waitingPending, withEvents } from "../test/fixtures.ts";
import { foldedReplaySteps } from "../test/replay.ts";
import { readRootVar } from "./rootVars.ts";

const END_TURN: ActionBody = { type: "endTurn" };

beforeEach(() => {
  vi.useFakeTimers();
});

afterEach(() => {
  cleanup();
  vi.useRealTimers();
  window.localStorage.clear();
  __resetSettingsForTests();
});

const START: GameEvent[] = [{ type: "turnStarted", player: "p1", turn: 3 }];
/** A hit on the viewer's hero: an event the runner animates, so it holds the view back. */
const HIT: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false };
const GIGA_HIT: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 25, combat: false };

describe("the board's moves wait for the board", () => {
  it("does not enable End turn for a view the runner is still holding back, and does once it drains", () => {
    const before = withEvents(baseView({ active: "p2" }), START);
    const onAction = vi.fn();
    const { rerender } = render(<Game view={before} legal={[]} onAction={onAction} />);
    expect(screen.getByTestId("end-turn")).toBeDisabled();

    // The opponent's last hit arrives with the viewer's turn: the board still shows the hit landing.
    const after = withEvents(baseView({ active: "p1" }), [...START, HIT]);
    rerender(<Game view={after} legal={[END_TURN]} onAction={onAction} />);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(screen.getByTestId("end-turn")).toBeDisabled();
    fireEvent.click(screen.getByTestId("end-turn"));
    expect(onAction).not.toHaveBeenCalled();

    // The runner drains; the board shows the new view, and its moves with it.
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
    fireEvent.click(screen.getByTestId("end-turn"));
    expect(onAction).toHaveBeenCalledWith(END_TURN);
  });

  it("offers the moves at once when nothing is animating", () => {
    render(<Game view={withEvents(baseView(), START)} legal={[END_TURN]} onAction={vi.fn()} />);
    expect(screen.getByTestId("end-turn")).not.toBeDisabled();
  });
});

describe("issue #57 damage feel", () => {
  it("keeps a GIGA damage number on the board for its full 1.2-second linger after the queue drains", () => {
    const before = withEvents(baseView(), START);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    const after = withEvents(baseView(), [...START, GIGA_HIT]);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);

    expect(screen.getByTestId("game")).toHaveAttribute("data-impact-tier", "giga");
    expect(screen.getByTestId("game")).toHaveAttribute("data-hit-stop", "true");
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.damage.durationMs);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(document.querySelector(".damage-pop")).toHaveTextContent("25");

    act(() => {
      vi.advanceTimersByTime(DAMAGE_FEEL.giga.numberLingerMs - ANIMATIONS.damage.durationMs);
    });
    expect(document.querySelector(".damage-pop")).toBeNull();
  });

  it("keeps hit-stop when Reduce motion drains damage entries synchronously", () => {
    writeSettings({ reduceMotion: true });
    const before = withEvents(baseView(), START);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={withEvents(baseView(), [...START, HIT])} legal={[]} onAction={vi.fn()} />);

    expect(screen.getByTestId("game")).toHaveAttribute("data-reduced-motion", "true");
    expect(screen.getByTestId("game")).toHaveAttribute("data-hit-stop", "true");
    act(() => {
      vi.advanceTimersByTime(DAMAGE_FEEL.normal.hitStopMs);
    });
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-hit-stop");
  });
});

describe("issue #185 unit slam on the board", () => {
  /** A MASSIVE Unit (20/20) landing in the opponent's lane 3: the newest view has it standing there. */
  function landing(): { before: PlayerView; after: PlayerView } {
    const giant = unit("p2", { instanceId: "giant", attack: 20, health: 20, maxHealth: 20 });
    const before = withEvents(baseView(), START);
    const event: GameEvent = { type: "summoned", player: "p2", instanceId: "giant", defId: giant.defId, row: "units", lane: 3 };
    const after = withEvents(
      baseView({ opponent: emptySide("p2", { hand: { count: 4 }, units: [null, null, giant, null, null] }) }),
      [...START, event],
    );
    return { before, after };
  }

  it("R701 marks the board with the landing's tier and side, hangs for its anticipation, then hit-stops on the landing", () => {
    const { before, after } = landing();
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);
    const game = screen.getByTestId("game");
    expect(game).toHaveAttribute("data-slam-tier", "massive");
    expect(game).toHaveAttribute("data-slam-side", "opponent");
    expect(readRootVar(game, "--slam-anticipation")).toBe(`${String(UNIT_SLAM.massive.anticipationMs)}ms`);
    // #3633: written onto its readers, never onto the root's own style, which restyled the whole game.
    expect(game.style.getPropertyValue("--slam-anticipation")).toBe("");
    expect(game).not.toHaveAttribute("data-hit-stop");
    act(() => {
      vi.advanceTimersByTime(UNIT_SLAM.massive.anticipationMs + ANIMATIONS.summoned.durationMs * SLAM_LAND_AT);
    });
    expect(game).toHaveAttribute("data-hit-stop", "true");
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(game).not.toHaveAttribute("data-slam-tier");
    expect(game).not.toHaveAttribute("data-hit-stop");
  });

  it("R701 under Reduce motion the landing still hit-stops, with no anticipation", () => {
    writeSettings({ reduceMotion: true });
    const { before, after } = landing();
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);
    const game = screen.getByTestId("game");
    expect(game).toHaveAttribute("data-hit-stop", "true");
    expect(game).not.toHaveAttribute("data-slam-tier");
  });
});

describe("issue #57 board-mounted match chrome", () => {
  it("mounts route chrome and action refusals inside the physical board rail", () => {
    render(
      <Game
        view={withEvents(baseView(), START)}
        legal={[]}
        onAction={vi.fn()}
        error="That action is no longer legal."
        boardRail={<a href="/">Back to lobby</a>}
        boardNotice={<p>Read-only replay</p>}
      />,
    );

    expect(screen.getByText("Back to lobby").closest(".board")).not.toBeNull();
    expect(screen.getByText("Read-only replay").closest(".board")).not.toBeNull();
    expect(screen.getByTestId("action-error").closest(".board")).not.toBeNull();
    expect(document.querySelector(".game > .game-error")).toBeNull();
  });
});

describe("#3633 the game screen's host", () => {
  it("#3633 Game marks its parent as a game host while mounted", () => {
    const shell = (showGame: boolean): ReactElement => (
      <div className="app-shell app-shell--wide" data-testid="shell">
        <nav className="screen-nav" />
        {showGame ? <Game view={withEvents(baseView(), START)} legal={[]} onAction={vi.fn()} /> : null}
      </div>
    );
    const { rerender, unmount } = render(shell(true));
    const host = screen.getByTestId("shell");
    expect(host).toHaveAttribute(GAME_HOST_ATTRIBUTE, "true");
    expect(document.querySelector(`.app-shell--wide[${GAME_HOST_ATTRIBUTE}] > .game`)).not.toBeNull();
    // The route takes the game down and keeps its shell: the shell is no game's host any more.
    rerender(shell(false));
    expect(host).not.toHaveAttribute(GAME_HOST_ATTRIBUTE);
    rerender(shell(true));
    expect(host).toHaveAttribute(GAME_HOST_ATTRIBUTE, "true");
    unmount();
    expect(host).not.toHaveAttribute(GAME_HOST_ATTRIBUTE);
  });
});

describe("#3633 the game root's marks", () => {
  it("#3633 Game marks its root where the stylesheets read a :has() on it: a play animating, the drag setting and the end", () => {
    const before = withEvents(baseView({ you: emptySide("p1", { hand: [card({ instanceId: "c9", defId: "core-002", cost: 1 })] }) }), START);
    const played: GameEvent = { type: "cardPlayed", player: "p1", instanceId: "c9", defId: "core-002", costPaid: 1 };
    const after = withEvents(before, [...START, played]);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    const game = screen.getByTestId("game");
    expect(game).not.toHaveAttribute("data-card-played");
    // The drag setting, as the board draws it (Board's `data-drag`).
    expect(game).toHaveAttribute("data-drag", screen.getByTestId(testid.board).getAttribute("data-drag") ?? "");
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);
    expect(game).toHaveAttribute("data-card-played", "true");
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(game).not.toHaveAttribute("data-card-played");
    act(() => {
      writeSettings({ dragToPlay: false });
    });
    expect(game).toHaveAttribute("data-drag", "off");
    expect(game).not.toHaveAttribute("data-game-over");
    rerender(<Game view={{ ...after, result: { winner: "p2", reason: "hero-death" } }} legal={[]} onAction={vi.fn()} />);
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(game).toHaveAttribute("data-game-over", "true");
    expect(document.querySelector('.game[data-game-over] > [data-testid="result-overlay"]')).not.toBeNull();
  });
});

function finished(over: Partial<PlayerView> = {}): PlayerView {
  return baseView({ result: { winner: "p2", reason: "hero-death" }, ...over });
}

describe("the result overlay", () => {
  it("says why in words, keeps Win / Loss / Draw and the engine's reason in data-reason", () => {
    render(<Game view={finished()} legal={[]} onAction={vi.fn()} />);
    const overlay = screen.getByTestId("result-overlay");
    expect(overlay).toHaveTextContent("Loss");
    expect(overlay).toHaveTextContent("Your hero has fallen.");
    expect(overlay).not.toHaveTextContent("hero-death");
    expect(overlay).toHaveAttribute("data-reason", "hero-death");
    expect(overlay).toHaveAttribute("data-outcome", "loss");
  });

  it("has a sentence for every reason, from either side", () => {
    const reasons = ["hero-death", "both-heroes-dead", "concede", "draw-accepted", "turn-cap", "disconnect", "match-ceiling", "alt-win", "won-by-effect", "voted-out", "both-voted-out", "fled", "fled-decisive"] as const;
    for (const reason of reasons) {
      for (const outcome of ["win", "loss", "draw"] as const) {
        const text = resultReason(outcome, reason);
        // A sentence, never the slug.
        expect(text, `${outcome} ${reason}`).toMatch(/^[A-Z].*\.$/);
        expect(text).not.toContain("-");
      }
    }
    expect(resultReason("win", "concede")).toBe("Your opponent conceded.");
    expect(resultReason("loss", "concede")).toBe("You conceded.");
  });

  it("R1843004 the other player's flight reads as a draw for the viewer, from viewFor alone", () => {
    // The view's p1 is the viewer: p2 fled, which is p2's win and the viewer's draw.
    render(<Game view={finished({ result: { winner: "p2", reason: "fled" } })} legal={[]} onAction={vi.fn()} />);
    const overlay = screen.getByTestId("result-overlay");
    expect(overlay).toHaveAttribute("data-outcome", "draw");
    expect(overlay).toHaveAttribute("data-reason", "fled");
    expect(overlay).toHaveTextContent("Draw");
    expect(overlay).toHaveTextContent("Your opponent escaped the universe. You draw.");
    cleanup();
    // A decisive flight is a loss, a removal by vote the removed player's loss, and two fleers a draw.
    render(<Game view={finished({ result: { winner: "p2", reason: "fled-decisive" } })} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-outcome", "loss");
    cleanup();
    render(<Game view={finished({ result: { winner: "p2", reason: "voted-out" } })} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-outcome", "loss");
    expect(screen.getByTestId("result-overlay")).toHaveTextContent("You were voted out of the game.");
    expect(resultReason("draw", "fled", "draw")).toBe("Both players escaped the universe.");
    expect(resultReason("win", "fled", "p1")).toBe("You escaped the universe. Your opponent draws.");
  });

  it("offers the route's actions in a panel over a dimmed board, and folds to a chip to view the board", () => {
    const onBack = vi.fn();
    render(
      <Game
        view={finished()}
        legal={[]}
        onAction={vi.fn()}
        resultActions={
          <button type="button" data-testid="route-back" onClick={onBack}>
            Back
          </button>
        }
      />,
    );
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-form", "panel");
    expect(document.querySelector(".result-scrim")).not.toBeNull();
    // Once it has come in, the route's first way on has the focus.
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(document.activeElement).toBe(screen.getByTestId("route-back"));
    fireEvent.click(screen.getByTestId("route-back"));
    expect(onBack).toHaveBeenCalledTimes(1);

    fireEvent.click(screen.getByTestId("result-view-board"));
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-form", "chip");
    expect(screen.getByTestId("result-overlay")).toHaveTextContent("Loss");
    expect(document.querySelector(".result-scrim")).toBeNull();

    fireEvent.click(screen.getByTestId("result-reopen"));
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-form", "panel");
  });

  it("waits out the effects layer's Victory / Defeat before it takes the pointer", () => {
    render(<Game view={finished()} legal={[]} onAction={vi.fn()} />);
    const overlay = screen.getByTestId("result-overlay");
    expect(overlay).toHaveAttribute("data-revealed", "false");
    expect(overlay.style.animationDelay).not.toBe("");
    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-revealed", "true");
  });

  it("stays a chip when the route draws its own result dialog", () => {
    render(<Game view={finished()} legal={[]} onAction={vi.fn()} resultForm="chip" />);
    expect(screen.getByTestId("result-overlay")).toHaveAttribute("data-form", "chip");
    expect(screen.queryByTestId("result-view-board")).toBeNull();
    expect(screen.queryByTestId("result-reopen")).toBeNull();
    expect(document.querySelector(".result-scrim")).toBeNull();
  });

  it("says Game over where the turn was, and a hero past lethal reads 0", () => {
    const view = finished({
      you: { ...baseView().you, hero: { ...baseView().you.hero, health: -6 } },
    });
    render(<Game view={view} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("turn-banner")).toHaveTextContent("Game over");
    expect(document.querySelector(".turn-plate")).toHaveTextContent("Game over");
    const health = document.querySelector('[data-testid="hero-you"] .hero-health, .hero .hero-health[data-health="-6"]');
    expect(health).not.toBeNull();
    expect(health).toHaveTextContent("0");
    expect(health).toHaveAttribute("data-health", "-6");
  });

  it("waits for the lethal burst, so the player sees how the game ended", () => {
    const before = withEvents(baseView({ active: "p2" }), START);
    const { rerender } = render(<Game view={before} legal={[]} onAction={vi.fn()} />);
    expect(screen.queryByTestId("result-overlay")).toBeNull();

    // The killing blow and the game over arrive together: the blow plays before the result covers it.
    const after = withEvents(finished(), [...START, HIT, { type: "gameOver", winner: "p2", reason: "hero-death" }]);
    rerender(<Game view={after} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("animation-queue")).toBeInTheDocument();
    expect(screen.queryByTestId("result-overlay")).toBeNull();

    act(() => {
      vi.advanceTimersByTime(10_000);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("result-overlay")).toBeInTheDocument();
  });
});

describe("each event animates once", () => {
  // p2's turn as p1 watches it: p2 drew a card face-down, then plays it. R97 judges a card by where
  // it sits now, so the second view's copy of that `drawn` names the card the first view's hid.
  const ended: GameEvent = { type: "turnEnded", player: "p1", turn: 3, unspentMana: 0 };
  const started: GameEvent = { type: "turnStarted", player: "p2", turn: 4 };
  const mana: GameEvent = { type: "manaChanged", player: "p2", current: 4, max: 4 };
  const hiddenDraw: GameEvent = { type: "drawn", player: "p2", instanceId: HIDDEN_ID, defId: HIDDEN_ID };
  const namedDraw: GameEvent = { type: "drawn", player: "p2", instanceId: "c26", defId: "core-004" };
  const play: GameEvent = { type: "cardPlayed", player: "p2", instanceId: "c26", defId: "core-004", costPaid: 3 };
  const land: GameEvent = { type: "summoned", player: "p2", instanceId: "c26", defId: "core-004", row: "units", lane: 1 };

  const drawnView = withEvents(baseView({ active: "p2", turn: 4 }), [ended, started, mana, hiddenDraw]);
  const playedView = withEvents(
    baseView({
      active: "p2",
      turn: 4,
      opponent: emptySide("p2", {
        hand: { count: 3 },
        units: [unit("p2", { instanceId: "c26", defId: "core-004" }), null, null, null, null],
      }),
    }),
    [{ ...ended }, { ...started }, { ...mana }, namedDraw, play, land],
  );

  /** The one entry the play is: in flight at once, and nothing after it. */
  function expectOnlyThePlay(): void {
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "cardPlayed");
    // The replay started with the turn banner and the opponent's library, which already played.
    expect(screen.getByTestId("turn-banner")).not.toHaveAttribute("data-animating");
    expect(screen.getByTestId("library-opponent")).not.toHaveAttribute("data-animating");
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.cardPlayed.durationMs);
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    expect(screen.getByTestId("turn-banner")).not.toHaveAttribute("data-animating");
  }

  it("R97 naming a card the last window drew face-down replays none of the window", () => {
    const { rerender } = render(<Game view={drawnView} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={playedView} legal={[]} onAction={vi.fn()} />);
    expectOnlyThePlay();
  });

  it("and not under StrictMode's doubled effects either", () => {
    const strict = (view: PlayerView): ReactElement => (
      <StrictMode>
        <Game view={view} legal={[]} onAction={vi.fn()} />
      </StrictMode>
    );
    const { rerender } = render(strict(drawnView));
    expect(screen.queryByTestId("animation-queue"), "the first view is the board as it has always been").toBeNull();
    rerender(strict(playedView));
    expectOnlyThePlay();
  });

  it("a runner rebuilt for Reduce motion mid-burst replays nothing when motion comes back", () => {
    const hit: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 3, combat: false };
    const again: GameEvent = { type: "damage", sourceId: null, targetId: "hero-p1", amount: 2, combat: false };
    const first = withEvents(baseView(), START);
    const { rerender } = render(<Game view={first} legal={[]} onAction={vi.fn()} />);
    rerender(<Game view={withEvents(baseView(), [...START, hit])} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "damage");

    act(() => {
      writeSettings({ reduceMotion: true });
    });
    expect(screen.queryByTestId("animation-queue")).toBeNull();
    act(() => {
      writeSettings({ reduceMotion: false });
    });
    expect(screen.queryByTestId("animation-queue"), "the rebuilt runner does not replay the hit").toBeNull();

    rerender(<Game view={withEvents(baseView(), [...START, hit, again])} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("animation-queue")).toHaveAttribute("data-animating", "damage");
    expect(screen.getByTestId("hero-you")).toHaveAttribute("data-animating", "damage");
    act(() => {
      vi.advanceTimersByTime(ANIMATIONS.damage.durationMs);
    });
    expect(screen.queryByTestId("animation-queue"), "one hit, one entry").toBeNull();
  });
});

describe("a replay board (#511)", () => {
  /** The first step of a folded game: the mulligan, a prompt a live board opens and lights its hand for. */
  const mulligan = (): PlayerView => {
    const view = foldedReplaySteps()[0]?.view;
    if (view?.pending === null || view === undefined) throw new Error("the folded game does not open on a prompt");
    return view;
  };

  it("a replay board opens no prompt where the same view live does", () => {
    const { unmount } = render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} />);
    expect(screen.getByTestId("prompt-scrim")).toBeInTheDocument();
    expect(document.querySelector("[data-glow]"), "live, the open prompt's cards glow").not.toBeNull();
    expect(screen.getByTestId("game")).not.toHaveAttribute("data-replay");
    unmount();

    render(<Game view={mulligan()} legal={[]} onAction={vi.fn()} replay />);
    expect(screen.queryByTestId("prompt-scrim")).toBeNull();
    expect(document.querySelector("[data-glow]")).toBeNull();
    expect(document.querySelector('[data-legal="true"]')).toBeNull();
    expect(screen.getByTestId("game")).toHaveAttribute("data-replay", "true");
  });

  it("R2211000 a replay board with replayPrompt draws the open mulligan read-only and folded, and no click sends anything", () => {
    const onAction = vi.fn();
    render(<Game view={mulligan()} legal={[]} onAction={onAction} replay replayPrompt />);

    const modal = screen.getByTestId("prompt-modal");
    expect(modal).toHaveAttribute("data-prompt-kind", "mulligan");
    expect(modal).toHaveAttribute("data-read-only", "true");
    expect(modal).toHaveAttribute("data-collapsed", "true");
    expect(document.querySelector("[data-glow]")).toBeNull();
    expect(screen.queryByTestId("prompt-submit")).toBeNull();

    fireEvent.click(screen.getByTestId("prompt-peek"));
    const options = [...document.querySelectorAll<HTMLElement>("[data-testid^='prompt-option-']")];
    expect(options.length).toBeGreaterThan(0);
    for (const option of options) {
      expect(option).toHaveAttribute("aria-disabled", "true");
      fireEvent.click(option);
    }
    expect(screen.queryByTestId("prompt-submit")).toBeNull();
    expect(onAction).not.toHaveBeenCalled();
  });

  it("a replay board sends no \"End turn automatically\" preference where a live one does", () => {
    // The step's view says the engine ends turns automatically; the player's setting says not to.
    act(() => {
      writeSettings({ autoEndTurn: false });
    });
    const live = vi.fn();
    const { unmount } = render(<Game view={mulligan()} legal={[]} onAction={live} />);
    expect(live).toHaveBeenCalledWith({ type: "setAutoEndTurn", enabled: false });
    unmount();

    const replay = vi.fn();
    render(<Game view={mulligan()} legal={[]} onAction={replay} replay />);
    expect(replay).not.toHaveBeenCalled();
  });
});

describe("R821000 a Plague Counter placement is answered by a click on the board", () => {
  const view = baseView({
    you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }),
    opponent: emptySide("p2", {
      hand: { count: 0 },
      units: [unit("p2", { instanceId: "e1" }), null, null, null, null],
      backrow: [null, null, faceDownBackrow, null, null],
    }),
    pending: pendingFor(
      "target",
      [
        { key: "instance:u1", label: "Mine", instanceId: "u1", defId: "core-002" },
        { key: "instance:e1", label: "Theirs", instanceId: "e1", defId: "core-002" },
        { key: "instance:t1", label: "Face-down card", instanceId: "t1", player: "p2", row: "backrow", lane: 3 },
      ],
      { prompt: "Place 2 Plague Counters on a permanent" },
    ),
  });
  const answer = (instanceId: string): ActionBody => ({
    type: "answer",
    choiceId: "ch1",
    selection: [{ pick: "instance", instanceId }],
  });

  it("answers with the card clicked, or with the face-down card whose zone is clicked, and opens no list", () => {
    const onAction = vi.fn();
    render(<Game view={view} legal={[]} onAction={onAction} />);

    expect(screen.getByTestId("prompt-modal").getAttribute("data-board-only")).toBe("true");
    expect(screen.queryByTestId("prompt-option-instance:e1")).toBeNull();

    fireEvent.click(screen.getByTestId(testid.card("e1")));
    expect(onAction).toHaveBeenLastCalledWith(answer("e1"));

    fireEvent.click(screen.getByTestId(testid.zone("opponent", "backrow", 3)));
    expect(onAction).toHaveBeenLastCalledWith(answer("t1"));
    expect(onAction).toHaveBeenCalledTimes(2);
  });

  it("answers nothing for a zone or a hero the prompt did not offer", () => {
    const onAction = vi.fn();
    render(<Game view={view} legal={[]} onAction={onAction} />);

    fireEvent.click(screen.getByTestId(testid.zone("opponent", "backrow", 4)));
    fireEvent.click(screen.getByTestId(testid.zone("opponent", "units", 2)));
    fireEvent.click(screen.getByTestId(testid.hero("opponent")));
    expect(onAction).not.toHaveBeenCalled();
  });
});

describe("a half-built play or attack is dropped at the turn change (#2924)", () => {
  const attacker = (): PlayerView =>
    baseView({
      you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }),
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "e1" }), null, null, null, null] }),
    });
  const attack: ActionBody = { type: "attack", attackerId: "u1", targetId: "e1" };

  it("drops the selected attacker when the turn changes and keeps it dropped next turn", () => {
    const onAction = vi.fn();
    const first = attacker();
    const { rerender } = render(<Game view={first} legal={[attack]} onAction={onAction} />);

    fireEvent.click(screen.getByTestId(testid.card("u1")));
    expect(screen.getByTestId(testid.card("u1"))).toHaveAttribute("data-selected", "true");
    expect(screen.getByTestId(testid.card("e1"))).toHaveAttribute("data-glow");

    rerender(<Game view={{ ...attacker(), active: "p2", turn: 4 }} legal={[]} onAction={onAction} />);
    expect(screen.getByTestId(testid.card("u1"))).not.toHaveAttribute("data-selected");

    rerender(<Game view={{ ...attacker(), turn: 5 }} legal={[attack]} onAction={onAction} />);
    expect(screen.getByTestId(testid.card("u1"))).not.toHaveAttribute("data-selected");
    expect(screen.getByTestId(testid.card("e1"))).not.toHaveAttribute("data-glow");

    fireEvent.click(screen.getByTestId(testid.card("e1")));
    expect(onAction).not.toHaveBeenCalled();
  });

  it("closes a play's own picker when the view moves to the opponent's turn", () => {
    const onAction = vi.fn();
    const mine = baseView({ you: emptySide("p1", { hand: [card({ instanceId: "h1" })] }) });
    const plays: ActionBody[] = [
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 1 } },
      { type: "play", instanceId: "h1", zone: { row: "units", lane: 2 } },
    ];
    const { rerender } = render(<Game view={mine} legal={plays} onAction={onAction} />);

    fireEvent.click(screen.getByTestId(testid.handCard("h1")));
    expect(screen.getByTestId("prompt-modal")).toHaveAttribute("data-prompt-source", "play");

    rerender(<Game view={{ ...mine, active: "p2", turn: 4 }} legal={[]} onAction={onAction} />);
    expect(screen.queryByTestId("prompt-scrim")).toBeNull();
  });

  it("keeps the selected attacker through view updates within the same turn", () => {
    const onAction = vi.fn();
    const first = attacker();
    const { rerender } = render(<Game view={first} legal={[attack]} onAction={onAction} />);

    fireEvent.click(screen.getByTestId(testid.card("u1")));
    expect(screen.getByTestId(testid.card("u1"))).toHaveAttribute("data-selected", "true");

    rerender(<Game view={{ ...first, clockMs: 59000 }} legal={[attack]} onAction={onAction} />);
    expect(screen.getByTestId(testid.card("u1"))).toHaveAttribute("data-selected", "true");

    rerender(<Game view={{ ...first, clockMs: 59000, pending: waitingPending }} legal={[attack]} onAction={onAction} />);
    expect(screen.getByTestId(testid.card("u1"))).toHaveAttribute("data-selected", "true");

    rerender(<Game view={{ ...first, clockMs: 59000, pending: null }} legal={[attack]} onAction={onAction} />);
    expect(screen.getByTestId(testid.card("u1"))).toHaveAttribute("data-selected", "true");
  });
});

describe("R3060000 a repeat of one intent against one view is sent once", () => {
  const clickEndTurn = () => fireEvent.click(screen.getByTestId("end-turn"));

  it("R3060000 two End turn clicks against one view send one endTurn, and a new view lets the next through", () => {
    const onAction = vi.fn();
    const { rerender } = render(<Game view={baseView()} legal={[END_TURN]} onAction={onAction} />);

    clickEndTurn();
    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith(END_TURN);

    rerender(<Game view={baseView()} legal={[END_TURN]} onAction={onAction} />);
    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(2);

    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(2);
  });

  it("R3060000 a refusal or an offline send releases the guard, and the error line clearing does not", () => {
    const onAction = vi.fn();
    const view = baseView();
    const board = (error: string | null) => <Game view={view} legal={[END_TURN]} onAction={onAction} error={error} />;
    const { rerender } = render(board(null));

    clickEndTurn();
    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(1);

    // A refusal arrives against the same view: the retry goes through, and its double click is one send.
    rerender(board("It is not your turn"));
    clickEndTurn();
    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(2);

    // net.ts clears the line on every send; that is no refusal, so the repeat stays dropped.
    rerender(board(null));
    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(2);

    // An offline send is an error too.
    rerender(board("not connected to the match"));
    clickEndTurn();
    expect(onAction).toHaveBeenCalledTimes(3);
  });

  it("R3060000 a double-tapped one-tap choice sends one answer, a different answer still goes, and a new view lets the same one go again", () => {
    const onAction = vi.fn();
    const asking = () =>
      baseView({
        pending: pendingFor(
          "answer",
          ["A", "B", "C", "D"].map((label) => ({ key: `mode:${label}`, label })),
          { prompt: "482 − 197 = ?" },
        ),
      });
    const answer = (option: string): ActionBody => ({
      type: "answer",
      choiceId: "ch1",
      selection: [{ pick: "mode", option }],
    });
    const board = (view: PlayerView) => (
      <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
        <Game view={view} legal={[]} onAction={onAction} />
      </CatalogContext.Provider>
    );
    const tap = (option: string) => fireEvent.click(screen.getByTestId(`prompt-option-mode:${option}`));
    const first = asking();
    const { rerender } = render(board(first));

    tap("B");
    tap("B");
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenLastCalledWith(answer("B"));

    tap("C");
    expect(onAction).toHaveBeenCalledTimes(2);
    expect(onAction).toHaveBeenLastCalledWith(answer("C"));

    rerender(board(asking()));
    tap("B");
    expect(onAction).toHaveBeenCalledTimes(3);
    expect(onAction).toHaveBeenLastCalledWith(answer("B"));
  });

  it("R3060000 a double-pressed mulligan Ready sends one mulligan", () => {
    const onAction = vi.fn();
    const hand = [
      card({ instanceId: "h1", defId: "core-002" }),
      card({ instanceId: "h2", defId: "core-019" }),
      card({ instanceId: "h3", defId: "core-004" }),
    ];
    const view = baseView({
      turn: 0,
      phase: "mulligan",
      you: emptySide("p1", { hand }),
      opponent: emptySide("p2", { hand: { count: 4 } }),
      pending: pendingFor(
        "mulligan",
        hand.map((c) => ({ key: c.instanceId, label: c.defId, instanceId: c.instanceId })),
        { min: 0, max: hand.length, prompt: "Choose the cards to keep; the rest are returned and redrawn" },
      ),
      mulligan: { youReady: false, opponentReady: false },
    });
    render(
      <CatalogContext.Provider value={lookupFromDefs(CATALOG)}>
        <Game view={view} legal={[]} onAction={onAction} />
      </CatalogContext.Provider>,
    );

    fireEvent.click(screen.getByTestId("prompt-submit"));
    fireEvent.click(screen.getByTestId("prompt-submit"));
    expect(onAction).toHaveBeenCalledTimes(1);
    expect(onAction).toHaveBeenCalledWith({ type: "mulligan", keep: ["h1", "h2", "h3"] });
  });

  it("R3060000 Concede and attacks are never dropped", () => {
    const concede: ActionBody = { type: "concede" };
    const onConcede = vi.fn();
    render(<Game view={baseView()} legal={[END_TURN, concede]} onAction={onConcede} />);
    for (let times = 0; times < 2; times += 1) {
      fireEvent.click(screen.getByTestId("concede"));
      fireEvent.click(screen.getByTestId("concede-confirm"));
    }
    expect(onConcede.mock.calls).toEqual([[concede], [concede]]);
    cleanup();

    const attack: ActionBody = { type: "attack", attackerId: "u1", targetId: "e1" };
    const view = baseView({
      you: emptySide("p1", { units: [unit("p1", { instanceId: "u1" }), null, null, null, null] }),
      opponent: emptySide("p2", { units: [unit("p2", { instanceId: "e1" }), null, null, null, null] }),
    });
    const onAttack = vi.fn();
    render(<Game view={view} legal={[attack]} onAction={onAttack} />);
    for (let times = 0; times < 2; times += 1) {
      fireEvent.click(screen.getByTestId(testid.card("u1")));
      fireEvent.click(screen.getByTestId(testid.card("e1")));
    }
    expect(onAttack.mock.calls).toEqual([[attack], [attack]]);
  });

  it("R3060000 a board whose views arrive at once (hotseat) never blocks", () => {
    const spy = vi.fn();
    function Seat() {
      const [, tick] = useState(0);
      return (
        <Game
          view={baseView()}
          legal={[END_TURN]}
          onAction={(body) => {
            spy(body);
            tick((n) => n + 1);
          }}
        />
      );
    }
    render(<Seat />);

    clickEndTurn();
    clickEndTurn();
    expect(spy).toHaveBeenCalledTimes(2);
  });
});
