// R3340000: a board card's art answers the player, in a real browser. apps/web's vitest suite proves which faces move and
// in which slots, the marks a unit's view draws and the rules artLive.css declares (cards/artLive.test.tsx,
// cards/art/wear.test.ts); what only a style engine can say is measured here, on a procedural card (Gary the Gambler), a
// Stinkyscape card (Tylannt) and a Training Grounds card (Celestial Caller): a pointer over it leans the subject toward it and
// scatters the motes, a press flinches it and a lift springs it back, an attack swings it and fires the motes, and a
// damaged, a buffed and a frozen unit's art wears its cracks, its glow and its frost. Each state is captured at a desktop
// viewport and at 390x844.
// Run: E2E_COMPONENT_PORT=5283 pnpm --dir e2e test:component --spec cypress/component/art-answers.cy.tsx

import { CATALOG } from "@jackioh/cards";
import type { UnitView } from "@jackioh/shared";

import { MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_SLOT_KINDS, resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

/** The three painters' cards: a procedural one, a Stinkyscape one and a Training Grounds one. */
const CARDS = [
  { id: "core-004", family: "procedural" },
  { id: "stinkyscape-006", family: "stinkyscape" },
  { id: "traininggrounds-016", family: "traininggrounds" },
] as const;

/** The desktop viewport (the config's) and a phone's, each with the box one face stands in. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720, box: 220 },
  { label: "phone", width: 390, height: 844, box: 150 },
] as const;

/** A face's box is 5:7. */
const BOX_ASPECT = 7 / 5;
/** How far a Buffed unit's attack stands over its printed one. */
const BUFF = 3;
const STAGE = '[data-which="stage"]';
const FACE = `${STAGE} .cf--minion`;
const ART = `${FACE} .cf-art`;

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

function clearPicks(): void {
  for (const { id } of CARDS) for (const kind of CARD_SLOT_KINDS) writeCardPick(id, kind, null);
}

/** A Unit on the board in a `.card.cf-host` as Card.tsx sets it (a bare box), worn as the viewer's own. */
function Stage({ id, box, over = {}, buffed = false, attacking = false }: { id: string; box: number; over?: Partial<UnitView>; buffed?: boolean; attacking?: boolean }) {
  const printed = defOf(id).base;
  const health = printed.health ?? 1;
  const u = unit("p1", { defId: id, attack: (printed.attack ?? 0) + (buffed ? BUFF : 0), health, maxHealth: health, ...over });
  const face = faceModel({
    defId: id,
    def: defOf(id),
    radiant: false,
    live: { attack: u.attack, health: u.health, maxHealth: u.maxHealth, keywords: u.keywords },
  });
  return (
    <div style={{ display: "flex", padding: 16, background: "#1b1f2b" }}>
      <div
        className="card cf-host"
        data-which="stage"
        tabIndex={0}
        style={{ width: box, height: box * BOX_ASPECT, flex: "none", position: "relative", display: "block", maxWidth: "none", padding: 0, border: 0, background: "none", boxShadow: "none", overflow: "visible" }}
      >
        <MinionFace face={face} unit={u} worn attacking={attacking} />
      </div>
    </div>
  );
}

/** An element's computed style, from the window it is in. */
function computed(el: HTMLElement): CSSStyleDeclaration {
  const win = el.ownerDocument.defaultView;
  if (win === null) throw new Error("no window");
  return win.getComputedStyle(el);
}

/** A computed individual transform property as a number: `none` is its rest, 1 for scale, 0 for rotate. */
function number(value: string, rest: number): number {
  return value === "none" ? rest : Number.parseFloat(value);
}

/** The viewport as Chrome painted it, saved under `name`. */
function capture(name: string): void {
  cy.screenshot(name, { capture: "viewport", overwrite: true, disableTimersAndAnimations: false });
}

beforeEach(() => {
  localStorage.clear();
  resetCosmeticsForTests();
  __resetSettingsForTests();
  clearPicks();
  writeCardSettings({ ...CARD_SETTINGS_DEFAULTS });
});

for (const viewport of VIEWPORTS) {
  for (const card of CARDS) {
    describe(`R3340000 ${card.family} art answers the player at ${viewport.label} size`, () => {
      const shot = (state: string): void => capture(`art-answers-${card.family}-${state}-${viewport.label}`);

      beforeEach(() => {
        cy.viewport(viewport.width, viewport.height);
      });

      it(`R3340000 ${card.family}: hover leans the subject and scatters the motes, a press flinches it and a lift springs it back`, () => {
        cy.mount(<Stage id={card.id} box={viewport.box} />);
        cy.get(FACE).should("have.attr", "data-art-hover", "false");
        // A Mythic's art moves at rest, in layers; any other card's holds still until the pointer comes.
        if (defOf(card.id).rarity === "Mythic") cy.get(ART).should("have.attr", "data-art-live", "true");
        else cy.get(ART).should("not.have.attr", "data-art-live");
        // The pointer comes in at the card's upper right.
        cy.get(STAGE).then(($stage) => {
          const rect = $stage[0]?.getBoundingClientRect();
          if (rect === undefined) throw new Error("no stage");
          cy.wrap($stage).trigger("pointerenter", { bubbles: false, force: true });
          cy.wrap($stage).trigger("pointermove", { clientX: rect.right - 4, clientY: rect.top + 4, force: true });
        });
        cy.get(FACE).should("have.attr", "data-art-hover", "true").and("have.attr", "data-art-motion", "animated");
        cy.get(`${ART}[data-art-live="true"]`).should("exist");
        cy.get(`${ART} > .cf-art-subject`).should(($subject) => {
          const style = computed($subject[0] as HTMLElement);
          expect(number(style.rotate, 0), "the subject turns toward the pointer").to.be.greaterThan(0);
          expect(style.translate, "and leans toward it").not.to.eq("none");
        });
        cy.get(`${ART} > .cf-art-motes`).should(($motes) => {
          expect(computed($motes[0] as HTMLElement).animationName, "the motes scatter").to.eq("cf-art-scatter");
        });
        shot("hover");

        cy.get(STAGE).trigger("pointerdown", { force: true });
        cy.get(FACE).should("have.attr", "data-art-press", "true");
        cy.get(`${ART} > .cf-art-subject`).should(($subject) => {
          expect(number(computed($subject[0] as HTMLElement).scale, 1), "the subject flinches inward").to.be.lessThan(1);
        });
        shot("press");
        cy.get(STAGE).trigger("pointerup", { force: true });
        cy.get(FACE).should("not.have.attr", "data-art-press");
        cy.get(`${ART} > .cf-art-subject`).should(($subject) => {
          expect(number(computed($subject[0] as HTMLElement).scale, 1), "and springs back").to.be.closeTo(1, 0.001);
        });

        // Leaving the card puts everything back.
        cy.get(STAGE).trigger("pointerleave", { bubbles: false, force: true });
        cy.get(FACE).should("have.attr", "data-art-hover", "false");
      });

      it(`R3340000 ${card.family}: an attack swings the subject toward its target and fires the motes`, () => {
        cy.mount(<Stage id={card.id} box={viewport.box} attacking />);
        cy.get(FACE).should("have.attr", "data-art-attacking", "true").and("have.attr", "data-art-motion", "animated");
        cy.get(`${ART}[data-art-live="true"]`).should("exist");
        cy.get(`${ART} > .cf-art-subject`).should(($subject) => {
          expect(computed($subject[0] as HTMLElement).animationName, "the subject swings").to.eq("cf-art-swing");
        });
        cy.get(`${ART} > .cf-art-motes`).should(($motes) => {
          expect(computed($motes[0] as HTMLElement).animationName, "the motes fire").to.eq("cf-art-fire");
        });
        shot("attack");
      });

      it(`R3340000 ${card.family}: a damaged unit's art shows cracks, a buffed one's a glow in its colours, a frozen one's frost`, () => {
        cy.mount(<Stage id={card.id} box={viewport.box} over={{ health: 1, maxHealth: 4 }} />);
        cy.get(`${ART}[data-art-damage="3"]`).should("exist");
        cy.get(`${ART} .cf-art-cracks path`).should("have.length", 3).and(($paths) => {
          const style = computed($paths[0] as unknown as HTMLElement);
          expect(style.stroke, "cracks are drawn").not.to.eq("none");
          expect(Number.parseFloat(style.strokeWidth), "with a stroke").to.be.greaterThan(0);
        });
        cy.get(`${ART} .cf-art-cracks`).should(($cracks) => {
          const box = $cracks[0]?.getBoundingClientRect();
          expect(box?.width, "across the art").to.be.greaterThan(20);
        });
        shot("damage");

        cy.mount(<Stage id={card.id} box={viewport.box} buffed />);
        cy.get(`${ART}[data-art-buffed="true"]`).should(($art) => {
          expect(($art[0] as HTMLElement).style.getPropertyValue("--art-glow"), "the glow is the card's colour").to.match(/^#[0-9a-f]{6}$/i);
        });
        cy.get(`${ART} .cf-art-glow`).should(($glow) => {
          expect(computed($glow[0] as HTMLElement).backgroundImage, "a glow is drawn").to.contain("radial-gradient");
        });
        shot("buff");

        cy.mount(<Stage id={card.id} box={viewport.box} over={{ frozen: true }} />);
        cy.get(`${ART}[data-art-frozen="true"]`).should("exist");
        cy.get(`${ART} .cf-art-frost`).should(($frost) => {
          expect(computed($frost[0] as HTMLElement).backgroundImage, "frost is drawn").to.contain("linear-gradient");
        });
        shot("freeze");

        // A whole, unbuffed, unfrozen unit wears nothing.
        cy.mount(<Stage id={card.id} box={viewport.box} />);
        cy.get(ART).should("exist");
        cy.get(`${ART} .cf-art-wear`).should("not.exist");
      });
    });
  }
}
