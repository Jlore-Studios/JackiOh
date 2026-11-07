// R3348001 (#3348): each playmat's corner props in a real browser, at the desktop and at 390x844. The unit
// tests prove what the layer writes (game/arena/CornerProps.test.tsx) and what the sheet states
// (arena/arenaCss.test.ts); what only a style engine can say is measured here: the props are 44 px or
// more each way, none covers a tile, a hand, a seat or the controls, and an escalated and a rare outcome
// are captured beside each mat's corners. At 390x844 the field keeps a strip of mat above and below its
// lanes for them (arena.css); a shorter phone has no height to spare and its props are hidden by the fit
// check, which the unit tests prove (R3348001, "Where they sit").

import { cosmeticsOf } from "../../../apps/web/src/cards/cosmetics.ts";
import { setArenaRandomForTests } from "../../../apps/web/src/game/arena/play.ts";
import { CORNER_OBSTACLES } from "../../../apps/web/src/game/arena/config.ts";
import Game from "../../../apps/web/src/game/Game.tsx";
import { fullBoardView } from "../../../apps/web/src/test/fixtures.ts";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

/** R2122000's `--hit-min`, as `HIT_MIN_PX` in e2e spec 47. */
const HIT_MIN_PX = 44;

/** A draw that takes the longest streak (7 taps) and never rolls the rare outcome. */
const LONGEST_STREAK = 0.99;
/** A draw that takes the shortest streak and always rolls the rare outcome. */
const ALWAYS_RARE = 0;

const MATS = cosmeticsOf("table").map((cosmetic) => cosmetic.id);

function Board({ table }: { table: string }) {
  return (
    <div className="app-shell app-shell--wide">
      <Game view={fullBoardView()} legal={[]} onAction={() => undefined} wearCosmetics deckCosmetics={{ table, field: null, cardBack: null }} />
    </div>
  );
}

const YOURS = '.corner-props[data-half="yours"] .corner-prop';
const THEIRS = '.corner-props[data-half="theirs"] .corner-prop';

/** The props of the viewer's half that are drawn: the fit check kept them and the style engine shows them. */
function shown(doc: Document): HTMLElement[] {
  return [...doc.querySelectorAll<HTMLElement>(YOURS)].filter(
    (prop) => prop.dataset.fit === "true" && getComputedStyle(prop).visibility !== "hidden",
  );
}

function overlap(a: DOMRect, b: DOMRect): boolean {
  return Math.min(a.right, b.right) - Math.max(a.left, b.left) > 0 && Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top) > 0;
}

for (const viewport of VIEWPORTS) {
  describe(`R3348001 the corner props at ${viewport.label} ${String(viewport.width)}x${String(viewport.height)}`, () => {
    // The runner's window is 720 high: a viewport capture of a taller page would be drawn at 720, where the
    // phone's props are hidden, so the phone's captures are of the runner, which shows the page at 390x844.
    const capture = viewport.height > 720 ? "runner" : "viewport";

    afterEach(() => {
      setArenaRandomForTests(null);
    });

    for (const table of MATS) {
      it(`R3348001 ${table}: two props a half, each at least ${String(HIT_MIN_PX)} px and clear of the lanes, the hands and the controls; an escalated and a rare outcome`, () => {
        cy.viewport(viewport.width, viewport.height);
        setArenaRandomForTests(() => LONGEST_STREAK);
        cy.mount(<Board table={table} />);

        cy.get(YOURS).should("have.length", 2);
        cy.get(THEIRS).should("have.length", 2);
        cy.get(`${YOURS}[data-fit="pending"]`).should("have.length", 0);

        cy.document().then((doc) => {
          const props = shown(doc);
          expect(props, `${table}: both props are drawn at ${viewport.label}`).to.have.length(2);
          const obstacles = [...doc.querySelectorAll(CORNER_OBSTACLES)].map((obstacle) => obstacle.getBoundingClientRect());
          for (const prop of props) {
            const box = prop.getBoundingClientRect();
            expect(box.width, `${prop.dataset.prop ?? ""} width`).to.be.at.least(HIT_MIN_PX);
            expect(box.height, `${prop.dataset.prop ?? ""} height`).to.be.at.least(HIT_MIN_PX);
            for (const obstacle of obstacles) expect(overlap(box, obstacle), `${prop.dataset.prop ?? ""} covers nothing`).to.equal(false);
          }
          expect(doc.documentElement.scrollWidth, "the page does not scroll sideways").to.be.at.most(viewport.width);
          cy.wrap(props[0]?.dataset.prop ?? "", { log: false }).as("tapped");
        });
        cy.screenshot(`arena-${table}-${viewport.label}`, { capture, overwrite: true });

        cy.get<string>("@tapped").then((id) => {
          expect(id, `${table}: a prop to tap at ${viewport.label}`).to.not.equal("");
          // The longest streak is 7 taps: the seventh is the payoff.
          for (let tap = 0; tap < 7; tap += 1) cy.get(`${YOURS}[data-prop="${id}"]`).click();
          cy.get(`${YOURS}[data-prop="${id}"]`).should("have.attr", "data-state", "escalate");
          cy.screenshot(`arena-${table}-${viewport.label}-escalated`, { capture, overwrite: true });

          // A command, not a bare call: Cypress queues the clicks above, and a call here would run before them.
          cy.then(() => {
            setArenaRandomForTests(() => ALWAYS_RARE);
          });
          cy.get(`${YOURS}[data-prop="${id}"]`).click();
          cy.get(`${YOURS}[data-prop="${id}"]`).should("have.attr", "data-state", "rare");
          cy.screenshot(`arena-${table}-${viewport.label}-rare`, { capture, overwrite: true });
        });
      });
    }
  });
}
