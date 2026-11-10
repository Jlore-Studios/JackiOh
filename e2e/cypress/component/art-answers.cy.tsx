// R3340000: a board card's art answers the player, in a real browser. apps/web's vitest suite proves which faces move and
// in which slots, the marks a unit's view draws and the rules artLive.css declares (cards/artLive.test.tsx,
// cards/art/wear.test.ts); what only a style engine can say is measured here, on a procedural card (Gary the Gambler), a
// Stinkyscape card (ZolarKeth) and a Training Grounds card (Celestial Caller): a pointer over it leans the subject toward it and
// scatters the motes, a press flinches it and a lift springs it back, an attack swings it and fires the motes, and a
// damaged, a buffed and a frozen unit's art wears its cracks, its glow and its frost. Each state is captured at a desktop
// viewport and at 390x844.
// R3350000: a Mythic whose normal picture has a diorama (Tylannt) draws its slices on the board in place of the live layers:
// its window swells on a hover and presses in on a press, and on an attack its slices over the farthest swing toward the
// target as a subject does (artLive.css), the farthest staying behind.
// Run: E2E_COMPONENT_PORT=5283 pnpm --dir e2e test:component --spec cypress/component/art-answers.cy.tsx

import { CATALOG } from "@jackioh/cards";
import type { UnitView } from "@jackioh/shared";
import type { CSSProperties } from "react";

import { MinionFace, faceModel } from "../../../apps/web/src/cards/index.ts";
import { CARD_SLOT_KINDS, resetCosmeticsForTests, writeCardPick } from "../../../apps/web/src/cards/cosmetics.ts";
import { CARD_SETTINGS_DEFAULTS, writeCardSettings } from "../../../apps/web/src/cards/settings.ts";
import { __resetSettingsForTests } from "../../../apps/web/src/settings/index.ts";
import { unit } from "../../../apps/web/src/test/fixtures.ts";

type CardDef = (typeof CATALOG)[string];

/** The three painters' cards: a procedural one, a Stinkyscape one and a Training Grounds one. */
const CARDS = [
  { id: "core-004", family: "procedural" },
  // R3350000: a Legendary, since a Mythic's moving picture on the board draws its diorama's slices instead (below).
  { id: "stinkyscape-026", family: "stinkyscape" },
  { id: "traininggrounds-016", family: "traininggrounds" },
] as const;

/** R3350000: a Mythic with a diorama, Tylannt, drawn by the Stinkyscape painter itself. */
const DIORAMA_MYTHIC = "stinkyscape-006";

/** The desktop viewport (the config's) and a phone's, each with the box one face stands in. */
const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720, box: 220 },
  { label: "phone", width: 390, height: 844, box: 150 },
] as const;

/** A face's box is 5:7. */
const BOX_ASPECT = 7 / 5;
/** How far a Buffed unit's attack stands over its printed one. */
const BUFF = 3;
/** The component attack's target vector, inherited by the subject as the live board's director writes it on the attacker. */
const ATTACK_TARGET_X_PX = 60;
const ATTACK_TARGET_Y_PX = -80;
/** The `cf-art-swing` keyframe at which its target-directed translate and scale are greatest. */
const ATTACK_PEAK_PROGRESS = 0.45;
const STAGE = '[data-which="stage"]';
const FACE = `${STAGE} .cf--minion`;
const ART = `${FACE} .cf-art`;

function defOf(id: string): CardDef {
  const def = CATALOG[id];
  if (def === undefined) throw new Error(`no ${id}`);
  return def;
}

function clearPicks(): void {
  for (const id of [...CARDS.map((card) => card.id), DIORAMA_MYTHIC]) for (const kind of CARD_SLOT_KINDS) writeCardPick(id, kind, null);
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
  const attackVector = attacking
    ? ({ "--fx-lunge-x": `${String(ATTACK_TARGET_X_PX)}px`, "--fx-lunge-y": `${String(ATTACK_TARGET_Y_PX)}px` } as CSSProperties)
    : {};
  return (
    <div style={{ display: "flex", padding: 16, background: "#1b1f2b" }}>
      <div
        className="card cf-host"
        data-which="stage"
        tabIndex={0}
        style={{ width: box, height: box * BOX_ASPECT, flex: "none", position: "relative", display: "block", maxWidth: "none", padding: 0, border: 0, background: "none", boxShadow: "none", overflow: "visible", ...attackVector }}
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

/** A computed individual `translate` property as its horizontal and vertical pixel components. */
function translation(value: string): { x: number; y: number } {
  if (value === "none") return { x: 0, y: 0 };
  const [x = "0", y = "0"] = value.split(/\s+/u);
  return { x: Number.parseFloat(x), y: Number.parseFloat(y) };
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
        // Its art holds still until the pointer comes (a Mythic's moves at rest, as its diorama: R3350000, below).
        cy.get(ART).should("not.have.attr", "data-art-live");
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
          const subject = $subject[0] as HTMLElement;
          const style = computed(subject);
          expect(style.animationName, "the subject swings").to.eq("cf-art-swing");
          const swing = subject.getAnimations().find((animation) => (animation as CSSAnimation).animationName === "cf-art-swing");
          expect(swing, "the subject's swing animation").not.to.equal(undefined);
          const duration = swing?.effect?.getTiming().duration;
          expect(duration, "the swing's duration").to.be.a("number");
          if (swing === undefined || typeof duration !== "number") return;
          swing.pause();
          swing.currentTime = duration * ATTACK_PEAK_PROGRESS;
          const peak = computed(subject);
          const toward = translation(peak.translate);
          expect(toward.x, "the subject moves right toward the target").to.be.greaterThan(0);
          expect(toward.y, "the subject moves up toward the target").to.be.lessThan(0);
          expect(number(peak.scale, 1), "the subject grows into the swing").to.be.greaterThan(1);
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

for (const viewport of VIEWPORTS) {
  describe(`R3350000 a board Mythic's diorama answers the player at ${viewport.label} size`, () => {
    const shot = (state: string): void => capture(`art-answers-diorama-${state}-${viewport.label}`);
    const SLICE = `${ART} > .cf-art-slice`;

    beforeEach(() => {
      cy.viewport(viewport.width, viewport.height);
    });

    it("R3350000 its window swells on a hover and presses in on a press, its slices drawn in place of the live layers", () => {
      cy.mount(<Stage id={DIORAMA_MYTHIC} box={viewport.box} />);
      // A Mythic's art moves at rest: its window draws the farthest slice and the rest stand over it, no live layers.
      cy.get(FACE).should("have.attr", "data-art-motion", "animated");
      cy.get(ART).should("have.attr", "data-art-slice", "0").and("not.have.attr", "data-art-live");
      cy.get(SLICE).should("have.length.greaterThan", 0);
      cy.get(`${ART} > .cf-art-subject, ${ART} > .cf-art-motes`).should("not.exist");
      cy.get(STAGE).then(($stage) => {
        const rect = $stage[0]?.getBoundingClientRect();
        if (rect === undefined) throw new Error("no stage");
        cy.wrap($stage).trigger("pointerenter", { bubbles: false, force: true });
        cy.wrap($stage).trigger("pointermove", { clientX: rect.right - 4, clientY: rect.top + 4, force: true });
      });
      cy.get(FACE).should("have.attr", "data-art-hover", "true");
      cy.get(ART).should(($art) => {
        expect(number(computed($art[0] as HTMLElement).scale, 1), "the window swells").to.be.greaterThan(1);
      });
      shot("hover");
      cy.get(STAGE).trigger("pointerdown", { force: true });
      cy.get(FACE).should("have.attr", "data-art-press", "true");
      cy.get(ART).should(($art) => {
        expect(number(computed($art[0] as HTMLElement).scale, 1), "and presses in").to.be.lessThan(1);
      });
      shot("press");
      cy.get(STAGE).trigger("pointerup", { force: true });
      cy.get(STAGE).trigger("pointerleave", { bubbles: false, force: true });
      cy.get(FACE).should("have.attr", "data-art-hover", "false");
    });

    it("R3350000 an attack swings its slices over the farthest toward the target, the farthest staying behind", () => {
      cy.mount(<Stage id={DIORAMA_MYTHIC} box={viewport.box} attacking />);
      cy.get(FACE).should("have.attr", "data-art-attacking", "true").and("have.attr", "data-art-motion", "animated");
      cy.get(ART).should("have.attr", "data-art-slice", "0").and(($art) => {
        expect(computed($art[0] as HTMLElement).animationName, "the farthest slice holds still").not.to.eq("cf-art-swing");
      });
      cy.get(SLICE).should(($slices) => {
        expect($slices.length, "slices over the farthest").to.be.greaterThan(0);
        for (const slice of $slices.toArray()) {
          expect(computed(slice).animationName, "each slice swings").to.eq("cf-art-swing");
        }
        const first = $slices[0] as HTMLElement;
        const swing = first.getAnimations().find((animation) => (animation as CSSAnimation).animationName === "cf-art-swing");
        expect(swing, "the slice's swing animation").not.to.equal(undefined);
        const duration = swing?.effect?.getTiming().duration;
        expect(duration, "the swing's duration").to.be.a("number");
        if (swing === undefined || typeof duration !== "number") return;
        swing.pause();
        swing.currentTime = duration * ATTACK_PEAK_PROGRESS;
        const peak = computed(first);
        const toward = translation(peak.translate);
        expect(toward.x, "the slices move right toward the target").to.be.greaterThan(0);
        expect(toward.y, "the slices move up toward the target").to.be.lessThan(0);
        expect(number(peak.scale, 1), "and grow into the swing").to.be.greaterThan(1);
      });
      shot("attack");
    });
  });
}
