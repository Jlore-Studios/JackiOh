// TEMPORARY CI-failure probe (not part of the change; deleted before finishing).
// Replicates e2e/support/component.tsx (global index.css FIRST, then the component
// tree, bare Game mount with a fixture PlayerView and no catalog provider) so
// headless Chromium measures the same layout and renders the same audio the Cypress
// component specs do. index.css must stay the first import: board.css's `.hero` row
// only beats index.css's landing-page `.hero` column by source order.
import "./src/index.css";
import { createRoot } from "react-dom/client";

import { SFX, SFX_IDS } from "./src/audio/sfx.ts";
import Game from "./src/game/Game.tsx";
import { fullBoardView } from "./src/test/fixtures.ts";

const mode = new URLSearchParams(window.location.search).get("mode") ?? "layout";

if (mode === "layout") {
  const view = fullBoardView();
  createRoot(document.getElementById("root")!).render(
    <div className="app-shell app-shell--wide">
      <Game view={view} legal={[]} onAction={() => undefined} />
    </div>,
  );
}

if (mode === "match" || mode === "hotseat") {
  const view = fullBoardView();
  const legal = [{ type: "endTurn" }, { type: "offerDraw" }, { type: "concede" }] as never[];
  const bar =
    mode === "match" ? (
      <header className="match-bar">
        <span>
          match <code>m-4f2c9e1a</code> · seat <code>p1</code> · turn 5 · <code>open</code>
        </span>
        <span className="clock">0:42 · grace 1:30</span>
      </header>
    ) : (
      <header className="hotseat-bar">
        <span>
          seed <code>b40-drag</code> · seat <code>p1</code> · turn 5 · active <code>p1</code>
        </span>
        <button type="button">Hand over to p2</button>
      </header>
    );
  createRoot(document.getElementById("root")!).render(
    <div className="app-shell app-shell--wide">
      {mode === "match" && <a href="/">← Back</a>}
      {bar}
      <Game view={view} legal={legal} onAction={() => undefined} />
    </div>,
  );
}

const SAMPLE_RATE = 44_100;
const TAIL_S = 0.25;

async function renderSfx(id: (typeof SFX_IDS)[number], params: Record<string, unknown>): Promise<{
  length: number;
  nonFinite: number;
  peak: number;
  tailPeak: number;
  durationMs: number;
}> {
  const spec = SFX[id];
  const seconds = spec.durationMs / 1000 + TAIL_S;
  const ctx = new OfflineAudioContext(1, Math.round(SAMPLE_RATE * seconds), SAMPLE_RATE);
  const out = ctx.createGain();
  out.gain.value = 1;
  out.connect(ctx.destination);
  spec.recipe(ctx, out, 0, params);
  const samples = (await ctx.startRendering()).getChannelData(0);
  const tailFrom = Math.ceil((SAMPLE_RATE * spec.durationMs) / 1000);
  let nonFinite = 0;
  let peak = 0;
  let tailPeak = 0;
  for (let i = 0; i < samples.length; i += 1) {
    const s = samples[i] ?? 0;
    if (!Number.isFinite(s)) nonFinite += 1;
    const m = Math.abs(s);
    if (m > peak) peak = m;
    if (i >= tailFrom && m > tailPeak) tailPeak = m;
  }
  return { length: samples.length, nonFinite, peak, tailPeak, durationMs: spec.durationMs };
}

if (mode === "practice") {
  const { card } = await import("./src/test/fixtures.ts");
  await import("./src/practice/practice.css");
  const ids = [
    "core-002", "core-019", "core-055", "core-077", "core-011",
    "core-013", "core-025", "core-054", "core-066", "core-068",
  ];
  const handSize = Math.min(10, Math.max(0, Number(new URLSearchParams(window.location.search).get("hand") ?? "10")));
  const view = fullBoardView();
  const hand = ids.slice(0, handSize).map((defId, index) => card({ defId, cost: index % 5 }));
  const withHand = { ...view, you: { ...view.you, hand } };
  createRoot(document.getElementById("root")!).render(
    <div className="app-shell app-shell--wide practice practice--game">
      <header className="practice-hud" data-testid="practice-hud">
        <span className="practice-hud__tier">Practice Easy</span>
      </header>
      <div className="practice-board practice-table" data-difficulty="easy" data-thinking="false">
        <Game view={withHand} legal={[]} onAction={() => undefined} />
      </div>
    </div>,
  );
}

if (mode === "deckbuilder") {
  const { default: DeckWorkshop } = await import("./src/game/deckbuilder/DeckWorkshop.tsx");
  const { CATALOG, CATALOG_VERSION } = await import("../../../packages/cards/src/catalog-data.ts");
  const { DECK_NAME_MAX_LENGTH, MAX_SAVED_DECKS, MAX_SAVED_TRIOS } = await import(
    "../../../apps/server/src/config.ts"
  );
  const deckable = Object.values(CATALOG).filter((def) => !def.token && !def.tags.includes("Token"));
  const DECK_SIZE_LOCAL = 20;
  const fullDecks = [0, 1, 2].map((deck) =>
    deckable.slice(deck * DECK_SIZE_LOCAL, (deck + 1) * DECK_SIZE_LOCAL).map((def) => def.id),
  );
  const ids = [
    "00000000-0000-4000-8000-000000000001",
    "00000000-0000-4000-8000-000000000002",
    "00000000-0000-4000-8000-000000000003",
  ];
  const data = {
    catalogVersion: CATALOG_VERSION,
    decks: fullDecks.map((cards, at) => ({
      id: ids[at] ?? ids[0],
      name: `Deck ${String(at + 1)}`,
      cards: [...cards],
      portrait: null,
      catalogVersion: CATALOG_VERSION,
      createdAt: at,
      updatedAt: at,
    })),
    trios: [
      {
        id: "00000000-0000-4000-8000-000000000004",
        name: "Trio 1",
        deckIds: [ids[0], ids[1], ids[2]],
        createdAt: 0,
        updatedAt: 0,
      },
    ],
    limits: { decks: MAX_SAVED_DECKS, trios: MAX_SAVED_TRIOS, nameLength: DECK_NAME_MAX_LENGTH },
  };
  const api = {
    putDeck: () => Promise.resolve({}),
    deleteDeck: () => Promise.resolve({}),
    putTrio: () => Promise.resolve({}),
    deleteTrio: () => Promise.resolve({}),
    importTrio: () => Promise.resolve({}),
  };
  createRoot(document.getElementById("root")!).render(
    <DeckWorkshop
      catalog={{ version: CATALOG_VERSION, cards: CATALOG }}
      collection={Object.fromEntries(deckable.map((def) => [def.id, 1]))}
      data={data}
      profileId="component-spec"
      api={api}
      storage={null}
      initialOpen={{ kind: "deck", id: ids[0] }}
    />,
  );
}

(window as unknown as { __probe: unknown }).__probe = { renderSfx, sfxIds: [...SFX_IDS] };
