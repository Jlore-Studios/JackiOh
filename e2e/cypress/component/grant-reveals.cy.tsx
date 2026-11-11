// R3349000: variant-level grant reveals. A duplicate's roll, a cosmetic pack's slot, a token
// redemption and a Chase Shard buy each reveal the card wearing the grant: first its previous look,
// then a wipe into the new level with its name and rarity beneath it, several grants playing in
// order rarest-last. Rare and Epic play on the panel, Legendary dims it and grows the card in,
// Mythic takes the screen; reduced motion shows stills, and a tap always moves on. A Mythic card
// pulled from a pack tears into the same takeover with its own art.
// Run: E2E_COMPONENT_PORT=5282 pnpm --dir e2e test:component --spec cypress/component/grant-reveals.cy.tsx

import { useState, type ReactElement } from "react";

import GrantReveal, { grantTestid, type GrantItem } from "../../../apps/web/src/economy/GrantReveal.tsx";
import { resetFxSettingsForTests, setFxSettings } from "../../../apps/web/src/fx/settings.ts";
import { openedPack } from "../../../apps/web/src/packs/fixtures.ts";
import { PackScene } from "../../../apps/web/src/packs/PackOpening.tsx";
import { GRANT_BEFORE_MS, GRANT_TAKEOVER_SHORT_MS, PACK_TEAR_MS, packTestid } from "../../../apps/web/src/packs/packs.ts";
import { __resetSettingsForTests, writeSettings } from "../../../apps/web/src/settings/index.ts";

const VIEWPORTS = [
  { label: "desktop", width: 1280, height: 720 },
  { label: "phone", width: 390, height: 844 },
] as const;

/** A Rare sketch, a Legendary full art and a Mythic signed full art, all on Big D-fender, rarest last. */
const GRANTS: readonly GrantItem[] = [
  { cosmeticId: "version-sketch", cardId: "core-001", rarity: "Rare" },
  { cosmeticId: "version-full-art", cardId: "core-001", rarity: "Legendary" },
  { cosmeticId: "version-full-art-signed", cardId: "core-001", rarity: "Mythic", chase: true },
];
const MYTHIC_GRANT: readonly GrantItem[] = [GRANTS[2]!];

function ts(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** The scene as every caller holds it (PackOpening, CosmeticPacks, the Collection's views): dropped once it calls `onDone`. */
function GrantOwner({ items, onDone }: { items: readonly GrantItem[]; onDone: () => void }): ReactElement | null {
  const [open, setOpen] = useState(true);
  if (!open) return null;
  return (
    <GrantReveal
      items={items}
      onDone={() => {
        setOpen(false);
        onDone();
      }}
    />
  );
}

function mountGrants(items: readonly GrantItem[], onDone: () => void): void {
  cy.mount(<GrantOwner items={items} onDone={onDone} />);
}

describe("R3349000 grant reveals", () => {
  beforeEach(() => {
    __resetSettingsForTests();
    resetFxSettingsForTests();
    cy.viewport(1280, 720);
  });

  after(() => {
    __resetSettingsForTests();
    resetFxSettingsForTests();
  });

  for (const vp of VIEWPORTS) {
    it(`R3349000 a Rare, a Legendary and a Mythic grant play rarest-last with the rarity's treatment at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      cy.clock();
      let done = false;
      mountGrants(GRANTS, () => {
        done = true;
      });

      // The Rare sketch on the panel: its previous look, then a wipe into the new level.
      cy.get(ts(grantTestid.scene)).should("have.attr", "data-treatment", "panel");
      cy.get(ts(grantTestid.scene)).should("have.attr", "data-stage", "before");
      cy.get(ts(grantTestid.item(0))).should("contain.text", "Sketch");
      cy.get(ts(grantTestid.name(0))).should("contain.text", "Rare");
      cy.tick(GRANT_BEFORE_MS);
      cy.get(ts(grantTestid.scene)).should("have.attr", "data-stage", "after");
      cy.get(ts(grantTestid.after(0))).should("exist");

      // The Legendary full art: the panel dims and the card grows in.
      cy.get(ts(grantTestid.scene)).click();
      cy.get(ts(grantTestid.scene)).should("have.attr", "data-treatment", "grow");
      cy.get(ts(grantTestid.name(1))).should("contain.text", "Legendary");
      cy.get(ts(grantTestid.scene)).click();
      cy.get(ts(grantTestid.scene)).should("have.attr", "data-stage", "after");

      // The Mythic chase takes the screen in the full-screen view; a tap moves on.
      cy.get(ts(grantTestid.scene)).click();
      cy.get(ts(grantTestid.scene)).should("have.attr", "data-treatment", "takeover");
      cy.get('[data-testid="fullscreen-art"]').should("be.visible");
      cy.get(ts(grantTestid.scene)).click();
      cy.then(() => {
        expect(done, "the queue ends after its last grant").to.equal(true);
      });
    });

    it(`R3349000 Skip ends the queue at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      let done = false;
      mountGrants(GRANTS, () => {
        done = true;
      });

      cy.get(ts(grantTestid.skip)).click();
      cy.then(() => {
        expect(done, "Skip ends the queue").to.equal(true);
      });
      cy.get(ts(grantTestid.scene)).should("not.exist");
    });

    it(`R3349000 a Mythic pull tears into its takeover at ${vp.label}`, () => {
      cy.viewport(vp.width, vp.height);
      cy.clock();
      cy.mount(
        <PackScene
          pack="Core"
          opened={openedPack()}
          busy={false}
          error={null}
          left={0}
          onTear={() => undefined}
          onNext={() => undefined}
          onClose={() => undefined}
        />,
      );
      // The tear's timer starts once the answered scene is drawn, so the clock moves only after it.
      cy.get(ts(packTestid.pack)).should("have.attr", "data-state", "tearing");
      cy.tick(PACK_TEAR_MS);
      // Slot 0 is the fixture's Mythic pull: the pack tears open into its takeover with its own art.
      cy.get(ts(packTestid.takeover)).should("have.attr", "data-card", "core-096");
      cy.get('[data-testid="fullscreen-art"]').should("be.visible");
      cy.get(`${ts(packTestid.takeover)} [data-testid="fullscreen-art"]`).click();
      cy.get(".pack-slot").should("have.length", 5);
      cy.get(ts(packTestid.slot(0))).should("have.attr", "data-revealed", "false");
    });
  }

  it("R3349000 under reduced motion each grant is a still before and after", () => {
    writeSettings({ reduceMotion: true });
    let done = false;
    mountGrants(GRANTS.slice(0, 2), () => {
      done = true;
    });

    cy.get(ts(grantTestid.scene)).should("have.attr", "data-motion", "reduce");
    cy.get(ts(grantTestid.scene)).should("have.attr", "data-stage", "before");
    cy.get(ts(grantTestid.scene)).should(($scene) => {
      const win = $scene[0]!.ownerDocument.defaultView!;
      const everything = [$scene[0]!, ...Array.from($scene[0]!.querySelectorAll("*"))];
      for (const element of everything) {
        for (const pseudo of [null, "::before", "::after"]) {
          expect(win.getComputedStyle(element, pseudo).animationName, `${element.tagName}.${element.className.toString()} ${pseudo ?? ""}`).to.equal(
            "none",
          );
        }
      }
    });
    cy.get(ts(grantTestid.scene)).click();
    cy.get(ts(grantTestid.scene)).should("have.attr", "data-stage", "after");
    cy.get(ts(grantTestid.scene)).click();
    cy.get(ts(grantTestid.name(1))).should("contain.text", "Legendary");
    cy.get(ts(grantTestid.scene)).click();
    cy.get(ts(grantTestid.scene)).click();
    cy.then(() => {
      expect(done, "taps move through every still").to.equal(true);
    });
  });

  it("R3349000 bigMoments off stills the full-screen grant", () => {
    setFxSettings({ bigMoments: "off" });
    cy.clock();
    let done = false;
    mountGrants(MYTHIC_GRANT, () => {
      done = true;
    });

    // Off: the Mythic grant is a still frame, the screen untaken; the hold still wipes into it.
    cy.get(ts(grantTestid.scene)).should("have.attr", "data-treatment", "takeover");
    cy.get('[data-testid="fullscreen-art"]').should("not.exist");
    cy.tick(GRANT_BEFORE_MS);
    cy.get(ts(grantTestid.scene)).should("have.attr", "data-stage", "after");
    cy.get(ts(grantTestid.after(0))).should("exist");
    cy.get(ts(grantTestid.scene)).click();
    cy.then(() => {
      expect(done, "a tap moves on").to.equal(true);
    });
  });

  it("R3349000 bigMoments short holds the full-screen grant briefly", () => {
    setFxSettings({ bigMoments: "short" });
    cy.clock();
    let done = false;
    mountGrants(MYTHIC_GRANT, () => {
      done = true;
    });

    cy.get('[data-testid="fullscreen-art"]').should("be.visible");
    cy.tick(GRANT_TAKEOVER_SHORT_MS);
    cy.then(() => {
      expect(done, "the short hold moves on by itself").to.equal(true);
    });
  });
});
