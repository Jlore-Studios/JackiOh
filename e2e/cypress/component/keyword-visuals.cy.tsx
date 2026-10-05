// R438: the keyword visuals on a board minion, in a real browser (issue #40).
//
// apps/web/src/cards/keywordVisuals.test.tsx proves the DOM and the stylesheet's text: which
// treatments a unit draws, the caps, the gate on `data-kw-motion`, the reduced-motion blocks. What
// only a layout engine can say is measured here, on MinionFace at three board sizes (a 1280x720
// desktop's 43x60 minion, a tablet's 58x80, the largest zone's 81x113):
//
//   - each keyword's treatment is drawn: a box of real size inside the card, and a glyph at least
//     GLYPH_MIN_PX wide;
//   - no treatment covers the attack or health number or the name: the middle of each is still the
//     gem or the plate itself, whatever keywords the unit has;
//   - a glyph never sits on the cost gem;
//   - loops run, at most AMBIENT_MAX treatments of a unit at once, and none at all once the settings
//     panel's Reduce motion is on (the media query's half is the stylesheet test's).
//
// `--expose shots=1` also saves pictures of every row, at the smallest and the largest size, three
// times larger (CSS zoom), for a person to look at.

import { CATALOG } from "../../../packages/cards/src/catalog-data.ts";
import { KEYWORD_KINDS, type Keyword, type KeywordKind } from "../../../packages/shared/src/catalog-types.ts";
import type { UnitView } from "../../../packages/shared/src/view.ts";
import { MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { AMBIENT_MAX } from "../../../apps/web/src/cards/keywordVisuals.ts";
import { writeSettings, __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

const SHOTS = ["1", "true"].includes(String(Cypress.expose("shots") ?? ""));

/** Minion boxes the board gives a unit (board.css `--card-h` and the zone's 5:7-ish shape). */
const SIZES = [
  { label: "desktop", width: 43, height: 60 },
  { label: "tablet", width: 58, height: 80 },
  { label: "large", width: 81, height: 113 },
] as const;

/** A glyph disc's floor (keywords.css `--kw-glyph`). */
const GLYPH_MIN_PX = 12;

/**
 * The card's top-right corner, kept clear of the glyph row for the switch button's glyph. #258 moved
 * the switch out of the card into its zone's corner (board.css `.board .zone > .switch-button`),
 * where on a tile the card fills its glyph still sits over this corner, so the room stays reserved:
 * `clamp(14px, 28%, 18px)` wide, 2px from the edge.
 */
const SWITCH = { min: 14, share: 0.28, max: 18, inset: 2 } as const;

/** How far a box may poke past the card's own edge (the card clips it anyway). */
const EDGE_SLACK_PX = 1;

const DEF_ID = "core-004";

function keywordOf(kind: KeywordKind): Keyword {
  switch (kind) {
    case "Armor":
    case "Lucky":
    case "Brittle":
    case "Spell Damage":
      return { kind, n: 2 };
    default:
      return { kind } as Keyword;
  }
}

type Row = { id: string; unit: Partial<UnitView> };

const ROWS: readonly Row[] = [
  ...KEYWORD_KINDS.map((kind) => ({
    id: kind,
    unit: { keywords: [keywordOf(kind)], armor: kind === "Armor" ? 2 : 0 },
  })),
  { id: "every keyword", unit: { keywords: KEYWORD_KINDS.map(keywordOf), armor: 2, counters: { plague: 2, grade: 3 } } },
  { id: "Poisonous Lifesteal Charge", unit: { keywords: [{ kind: "Poisonous" }, { kind: "Lifesteal" }, { kind: "Charge" }] } },
  { id: "Taunt Reborn Pierce Cleave", unit: { keywords: [{ kind: "Taunt" }, { kind: "Reborn" }, { kind: "Pierce" }, { kind: "Cleave" }] } },
  { id: "Brittle 1", unit: { keywords: [{ kind: "Brittle", n: 3 }], brittle: 1 } },
  { id: "Stack, 2 buried", unit: { keywords: [{ kind: "Stack" }], buried: 2 } },
];

/**
 * Tall enough for every row of the grid, so a hit test can reach the last one: a row is the largest
 * minion (113px) and its gap, and a new keyword adds a row, so the height follows the rows rather than
 * being a number to bump (R636 and R637 added two and pushed the last row past 3400).
 */
const GRID_ROW_PX = 131;
const GRID_SLACK_PX = 100;
/** The viewport stays under this however many rows there are: the hit test scrolls each cell into view. */
const VIEWPORT_MAX_PX = 4000;
const GRID_VIEWPORT = { width: 1000, height: Math.min(VIEWPORT_MAX_PX, ROWS.length * GRID_ROW_PX + GRID_SLACK_PX) } as const;

function Minion({ row, width, height }: { row: Row; width: number; height: number }) {
  const u = unit("p1", { defId: DEF_ID, attack: 3, health: 4, maxHealth: 4, ...row.unit });
  const face = faceModel({
    defId: DEF_ID,
    def: CATALOG[DEF_ID],
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  return (
    <div data-row={row.id} data-size={`${width}x${height}`} style={{ width, height, flex: "none", position: "relative" }}>
      <MinionFace face={face} unit={u} />
    </div>
  );
}

type GridProps = { zoom?: number; rows?: readonly Row[]; sizes?: readonly (typeof SIZES)[number][] };

function Grid({ zoom = 1, rows = ROWS, sizes = SIZES }: GridProps) {
  return (
    <div style={{ background: "#1b1f2b", padding: 8, zoom }}>
      {rows.map((row) => (
        <div key={row.id} style={{ display: "flex", alignItems: "flex-start", gap: 6, marginBottom: 6 }}>
          <div style={{ width: 110, color: "#dfe4ee", font: "9px sans-serif" }}>{row.id}</div>
          {sizes.map((size) => (
            <Minion key={size.label} row={row} width={size.width} height={size.height} />
          ))}
        </div>
      ))}
    </div>
  );
}

/** Hit-testing needs the face to take the pointer, which it never does in the app. Test page only. */
function HitTestable() {
  return <style>{".cf, .cf * { pointer-events: auto !important; }"}</style>;
}

const overlap = (a: DOMRect, b: DOMRect): number =>
  Math.max(0, Math.min(a.right, b.right) - Math.max(a.left, b.left)) * Math.max(0, Math.min(a.bottom, b.bottom) - Math.max(a.top, b.top));

describe("R438 keyword visuals on the board minion", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    cy.viewport(GRID_VIEWPORT.width, GRID_VIEWPORT.height);
  });

  after(() => {
    __resetSettingsForTests();
  });

  it("R438 every keyword's treatment is drawn inside its card at every size, and a glyph is legible", () => {
    cy.mount(<Grid />);
    cy.get("[data-row] .cf").should("have.length", ROWS.length * SIZES.length);
    cy.document().should((doc) => {
      const problems: string[] = [];
      for (const kind of KEYWORD_KINDS) {
        for (const size of SIZES) {
          const cell = doc.querySelector<HTMLElement>(`[data-row="${kind}"][data-size="${size.width}x${size.height}"]`);
          const fx = cell?.querySelector<HTMLElement>(`[data-keyword-fx="${kind}"]`) ?? null;
          const where = `${kind} at ${size.label}`;
          if (cell === null || fx === null) {
            problems.push(`${where}: no treatment`);
            continue;
          }
          const box = fx.getBoundingClientRect();
          const card = cell.getBoundingClientRect();
          if (box.width < 4 || box.height < 4) problems.push(`${where}: ${box.width}x${box.height}`);
          if (box.left < card.left - EDGE_SLACK_PX || box.right > card.right + EDGE_SLACK_PX) problems.push(`${where}: outside the card`);
          if (fx.getAttribute("data-kw-layer") === "glyph") {
            if (box.width < GLYPH_MIN_PX - 0.5) problems.push(`${where}: glyph ${box.width}px wide`);
            const gem = cell.querySelector(".cost-gem")?.getBoundingClientRect();
            if (gem !== undefined && overlap(box, gem) > 1) problems.push(`${where}: glyph on the cost gem`);
          }
        }
      }
      // Every glyph a row shows, the last included, stops short of the switch button's corner.
      for (const cell of doc.querySelectorAll<HTMLElement>("[data-row]")) {
        const card = cell.getBoundingClientRect();
        const switchLeft = card.right - SWITCH.inset - Math.min(SWITCH.max, Math.max(SWITCH.min, card.width * SWITCH.share));
        for (const glyph of cell.querySelectorAll<HTMLElement>(".kw-glyphs > .kw-fx")) {
          if (getComputedStyle(glyph).display === "none") continue;
          const box = glyph.getBoundingClientRect();
          if (box.right > switchLeft + EDGE_SLACK_PX) {
            problems.push(`${cell.getAttribute("data-row") ?? "?"} at ${cell.getAttribute("data-size") ?? "?"}: ${glyph.getAttribute("data-keyword-fx") ?? "?"} under the switch button`);
          }
        }
      }
      expect(problems).to.deep.equal([]);
    });
  });

  it("R438 no treatment covers the attack, the health or the name, whatever keywords the unit has", () => {
    cy.mount(
      <>
        <HitTestable />
        <Grid />
      </>,
    );
    cy.get("[data-row] .cf").should("have.length", ROWS.length * SIZES.length);
    cy.document().should((doc) => {
      const problems: string[] = [];
      for (const cell of doc.querySelectorAll<HTMLElement>("[data-row]")) {
        const where = `${cell.getAttribute("data-row") ?? "?"} at ${cell.getAttribute("data-size") ?? "?"}`;
        for (const selector of [".stat-attack", ".stat-health", ".card-name"]) {
          const el = cell.querySelector<HTMLElement>(selector);
          if (el === null) {
            problems.push(`${where}: no ${selector}`);
            continue;
          }
          // A point outside the viewport hits nothing, so bring the element into it first.
          el.scrollIntoView({ block: "center", inline: "center" });
          const r = el.getBoundingClientRect();
          const hit = doc.elementFromPoint(r.left + r.width / 2, r.top + r.height / 2);
          if (hit === null || !(hit === el || el.contains(hit))) {
            const cover = hit?.closest("[data-keyword-fx]")?.getAttribute("data-keyword-fx") ?? hit?.className ?? "nothing";
            problems.push(`${where}: ${selector} is covered by ${String(cover)}`);
          }
        }
      }
      expect(problems).to.deep.equal([]);
    });
  });

  it(`R438 loops run, at most ${String(AMBIENT_MAX)} treatments of a unit at once`, () => {
    cy.mount(<Grid />);
    cy.get("[data-row] .cf").should("have.length", ROWS.length * SIZES.length);
    cy.document().should((doc) => {
      let running = 0;
      for (const cell of doc.querySelectorAll<HTMLElement>("[data-row]")) {
        const moving = [...cell.querySelectorAll<HTMLElement>("[data-keyword-fx]")].filter((fx) =>
          fx.getAnimations({ subtree: true }).some((animation) => animation.playState === "running"),
        );
        running += moving.length;
        expect(moving.length, `${cell.getAttribute("data-row") ?? "?"} loops`).to.be.at.most(AMBIENT_MAX);
        for (const fx of moving) expect(fx.getAttribute("data-kw-motion")).to.equal("on");
      }
      expect(running, "loops on the grid").to.be.greaterThan(0);
    });
  });

  it("R438 the settings panel's Reduce motion stops every loop and keeps every mark", () => {
    writeSettings({ reduceMotion: true });
    cy.mount(<Grid />);
    cy.get("html").should("have.attr", "data-reduce-motion", "true");
    cy.document().should((doc) => {
      const moving = [...doc.querySelectorAll<HTMLElement>("[data-keyword-fx]")].filter((fx) =>
        fx.getAnimations({ subtree: true }).some((animation) => animation.playState === "running"),
      );
      expect(moving.map((fx) => fx.getAttribute("data-keyword-fx"))).to.deep.equal([]);
      for (const kind of KEYWORD_KINDS) {
        expect(doc.querySelector(`[data-row="${kind}"] [data-keyword-fx="${kind}"]`), kind).to.not.equal(null);
      }
    });
  });

  if (SHOTS) {
    // A board's smallest minion and its largest, three times larger (CSS zoom, so every px floor
    // scales with them and the picture is the small card magnified), two rows to a picture, which
    // is what fits the headless browser's 720px shot.
    const CHUNK = 2;
    const ZOOM = 3;
    for (let start = 0; start < ROWS.length; start += CHUNK) {
      it(`R438 pictures: rows ${String(start + 1)} to ${String(Math.min(ROWS.length, start + CHUNK))}, three times larger`, () => {
        const rows = ROWS.slice(start, start + CHUNK);
        const sizes = [SIZES[0], SIZES[2]];
        cy.viewport(800, 720);
        cy.mount(<Grid zoom={ZOOM} rows={rows} sizes={sizes} />);
        cy.get("[data-row] .cf").should("have.length", rows.length * sizes.length);
        cy.screenshot(`keyword-visuals-3x-${String(start / CHUNK + 1)}`, { capture: "viewport" });
      });
    }
  }
});
