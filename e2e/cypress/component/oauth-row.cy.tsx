// R3217000: Google's and Roblox's sign-in buttons are one size and share one row, which jsdom cannot
// measure. The real markup (`OAuthButtons`) is mounted in `/login`'s own board with the real sheets
// (`auth.css`, `tavern.css`), Google's as R666's styled fallback (GIS needs the network and draws
// its own button, which `GIS_BUTTON.width` sizes to the same width). `should` retries through relayout.

import type { ReactElement } from "react";

import OAuthButtons from "../../../apps/web/src/auth/OAuthButtons.tsx";
import { OAUTH_BUTTON_HEIGHT_PX, OAUTH_BUTTON_WIDTH_PX } from "../../../apps/web/src/auth/googleIdentity.ts";
import { loginTestid } from "../../../apps/web/src/auth/testids.ts";
import type { OAuthProvider } from "../../../apps/web/src/net/auth.ts";

import "../../../apps/web/src/auth/auth.css";
import "../../../apps/web/src/auth/tavern.css";

/** The most two boxes may differ by and still read as one size. */
const SIZE_TOLERANCE_PX = 1;

const DESKTOP = { width: 1280, height: 720 } as const;
const PHONE = { width: 390, height: 844 } as const;

function byTestid(testid: string): string {
  return `[data-testid="${testid}"]`;
}

/** `/login`'s board around the row, as `routes/login.tsx` draws it. */
function Harness({ providers }: { providers: readonly OAuthProvider[] }): ReactElement {
  return (
    <div className="app-shell auth-screen tavern one-screen">
      <section className="panel panel--auth panel--split one-screen__body">
        <div className="auth-board__body">
          <div className="auth-oauth" role="group" aria-label="Other ways to sign in">
            <OAuthButtons
              providers={providers}
              googleClient={null}
              busy={false}
              onContinue={() => undefined}
              onGoogleCredential={() => undefined}
            />
          </div>
        </div>
      </section>
    </div>
  );
}

/** The two buttons' boxes, read once the row has laid out. */
function pair(row: Element): { google: DOMRect; roblox: DOMRect } {
  const box = (provider: OAuthProvider): DOMRect => {
    const button = row.querySelector(`[data-provider="${provider}"]`);
    expect(button, `${provider}'s button is in the row`).to.not.eq(null);
    if (button === null) throw new Error(`${provider}'s button is not in the row`);
    return button.getBoundingClientRect();
  };
  return { google: box("google"), roblox: box("custom:roblox") };
}

function expectSameSize(google: DOMRect, roblox: DOMRect): void {
  expect(Math.abs(google.height - roblox.height), "the same height").to.be.at.most(SIZE_TOLERANCE_PX);
  expect(Math.abs(google.width - roblox.width), "the same width").to.be.at.most(SIZE_TOLERANCE_PX);
}

describe("R3217000 Google's and Roblox's buttons, one size, one row", () => {
  it("R3217000 Google and Roblox match, on one row", () => {
    cy.viewport(DESKTOP.width, DESKTOP.height);
    cy.mount(<Harness providers={["google", "custom:roblox"]} />);

    cy.get(byTestid(loginTestid.oauthRow)).should(($row) => {
      const row = $row[0];
      expect(row, "the row is mounted").to.not.eq(undefined);
      if (row === undefined) return;
      const { google, roblox } = pair(row);
      expectSameSize(google, roblox);
      expect(Math.abs(google.top - roblox.top), "the same top").to.be.at.most(SIZE_TOLERANCE_PX);
      expect(google.right, "Google ends before Roblox starts, so they do not overlap").to.be.at.most(roblox.left);
      expect(google.height, "the height GIS's large button has").to.be.closeTo(OAUTH_BUTTON_HEIGHT_PX, SIZE_TOLERANCE_PX);
      expect(google.width, "the width GIS is drawn at").to.be.closeTo(OAUTH_BUTTON_WIDTH_PX, SIZE_TOLERANCE_PX);
      // An even gap, and the pair centered in the row.
      // Neither label is cut short by its ellipsis.
      for (const label of row.querySelectorAll(".auth-oauth-label")) {
        expect(label.scrollWidth, `${label.textContent ?? ""} fits its button`).to.be.at.most(label.clientWidth);
      }
      const gap = roblox.left - google.right;
      expect(gap, "a gap between them").to.be.greaterThan(0);
      const rowBox = row.getBoundingClientRect();
      expect(
        Math.abs(google.left - rowBox.left - (rowBox.right - roblox.right)),
        "the pair is centered in its row",
      ).to.be.at.most(SIZE_TOLERANCE_PX);
    });
  });

  it("R3217000 on a phone they match, side by side or stacked", () => {
    cy.viewport(PHONE.width, PHONE.height);
    cy.mount(<Harness providers={["google", "custom:roblox"]} />);

    cy.get(byTestid(loginTestid.oauthRow)).should(($row) => {
      const row = $row[0];
      expect(row, "the row is mounted").to.not.eq(undefined);
      if (row === undefined) return;
      const { google, roblox } = pair(row);
      expectSameSize(google, roblox);
      const sideBySide = Math.abs(google.top - roblox.top) <= SIZE_TOLERANCE_PX && google.right <= roblox.left;
      const stacked = roblox.top >= google.bottom;
      expect(sideBySide || stacked, "side by side on one row, or Roblox wrapped below Google").to.eq(true);
      expect(row.ownerDocument.documentElement.scrollWidth, "the page does not scroll sideways").to.be.at.most(
        row.ownerDocument.defaultView?.innerWidth ?? PHONE.width,
      );
    });
  });

  for (const provider of ["custom:roblox", "google"] as const) {
    it(`R3217000 a lone provider is centered: ${provider}`, () => {
      cy.viewport(DESKTOP.width, DESKTOP.height);
      cy.mount(<Harness providers={[provider]} />);

      cy.get(`${byTestid(loginTestid.oauth)}[data-provider="${provider}"]`).should(($button) => {
        const button = $button[0];
        expect(button, "the button is mounted").to.not.eq(undefined);
        if (button === undefined) return;
        const group = button.closest(".auth-oauth");
        expect(group, "the button is in the providers' group").to.not.eq(null);
        if (group === null) return;
        const at = button.getBoundingClientRect();
        const groupBox = group.getBoundingClientRect();
        expect(
          Math.abs((at.left + at.right) / 2 - (groupBox.left + groupBox.right) / 2),
          "the button sits in the middle of its group",
        ).to.be.at.most(SIZE_TOLERANCE_PX);
        expect(at.width, "the shared width").to.be.closeTo(OAUTH_BUTTON_WIDTH_PX, SIZE_TOLERANCE_PX);
      });
    });
  }
});
