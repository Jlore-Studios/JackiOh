// B53: browser layout of the HUD mute toggle inside the `.app-shell` every Game screen uses.
// CSS specificity against `index.css` cannot be verified in jsdom.

import AudioToggle from "../../../apps/web/src/audio/AudioToggle.tsx";

const SIZE_PX = 44;
const ICON_PX = 22;
const VIEWPORTS = [
  [1280, 720],
  [390, 844],
] as const;

describe("polish 2 — B53 the mute toggle inside .app-shell", () => {
  for (const [width, height] of VIEWPORTS) {
    it(`B53 at ${String(width)}x${String(height)} it is a ${String(SIZE_PX)} px circle with a ${String(ICON_PX)} px icon`, () => {
      cy.viewport(width, height);
      cy.mount(
        <div className="app-shell app-shell--wide">
          <AudioToggle />
        </div>,
      );
      cy.get('[data-testid="audio-toggle"]').then(($button) => {
        const button = $button[0] as HTMLButtonElement;
        const style = getComputedStyle(button);
        const box = button.getBoundingClientRect();
        const icon = button.querySelector("svg")?.getBoundingClientRect();
        cy.task("layout:report", {
          b53: `${String(width)}x${String(height)}`,
          borderRadius: style.borderRadius,
          padding: style.padding,
          box: [box.width, box.height],
          icon: icon === undefined ? null : [icon.width, icon.height],
        });
        expect(style.borderRadius, "a circle").to.eq("50%");
        expect(style.padding, "no padding squeezing the icon").to.eq("0px");
        expect(box.width).to.be.closeTo(SIZE_PX, 0.5);
        expect(box.height).to.be.closeTo(SIZE_PX, 0.5);
        expect(icon?.width).to.be.closeTo(ICON_PX, 0.5);
        expect(icon?.height).to.be.closeTo(ICON_PX, 0.5);
      });
    });
  }
});
