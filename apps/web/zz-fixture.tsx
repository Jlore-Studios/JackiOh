import { createRoot } from "react-dom/client";
import "./src/index.css";
import Game from "./src/game/Game.tsx";
import { fullBoardView, heroPower } from "./src/test/fixtures.ts";

const q = new URLSearchParams(window.location.search);
const base = fullBoardView();
const act = { ability: "", label: "Deal 1 damage", usesLeft: 1, usable: true } as never;
const view = q.has("act")
  ? { ...base, you: { ...base.you, units: base.you.units.map((u, i) => (u !== null && (i === 0 || i === 1 || i === 3) ? { ...u, activations: [act] } : u)) } }
  : base;
const rail = q.has("rail") ? (
  <>
    <nav className="row screen-nav">
      <button type="button" className="link-button" data-testid="nav-back">← Back</button>
    </nav>
    <p className="match-ranks" data-testid="match-ranks">
      <span className="match-ranks__full">Ranked match · MihoMain (you) Raisin vs LazyPirate Raisin</span>
      <span className="match-ranks__short">
        <span className="match-ranks__label">Ranked · </span>
        <span className="username"><span className="username__base">MihoMain</span></span> (you) <span className="rank-crest" style={{ width: 16, height: 16, display: "inline-block" }}>◆</span>{" "}
        <span className="match-ranks__tier">Normal II</span> vs <span className="username"><span className="username__base">LazyPirate</span></span>{" "}
        <span className="rank-crest" style={{ width: 16, height: 16, display: "inline-block" }}>◆</span> <span className="match-ranks__tier">Raisin</span>
      </span>
    </p>
  </>
) : undefined;
void heroPower;
createRoot(document.querySelector("[data-cy-root]")!).render(
  <div className="app-shell app-shell--wide">
    <Game view={view} legal={[]} onAction={() => undefined} boardRail={rail} phoneRailTop={q.has("top")} />
  </div>,
);
