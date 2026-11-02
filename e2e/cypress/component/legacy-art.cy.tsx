// R2724000: each Training Grounds Legacy card draws its own yard picture beside its Core card's.
// Both pictures paint and differ in their pixels; the screenshot is for a person to look at.

import { CATALOG } from "@jackioh/cards";

import { CardFace, FACE_ASPECT, faceModel } from "../../../apps/web/src/cards/index.ts";

type CardDef = (typeof CATALOG)[string];

/** How long a face may take to settle before its art must be painted. */
const SETTLE_TIMEOUT_MS = 20_000;
/** Each face's width on the page; the height follows the faces' aspect. */
const FACE_WIDTH_PX = 140;
/** Each picture is sampled down to this square before its pixels are compared. */
const SAMPLE_PX = 48;
/** Two channel values within this distance count as the same. */
const CHANNEL_SLACK = 24;
/** At least this share of a Legacy picture's pixels differs from its Core card's. */
const MIN_CHANGED_SHARE = 0.25;

/** The twenty Legacy defs. */
const LEGACY: readonly CardDef[] = Object.values(CATALOG).filter((def) => def.tags.includes("Legacy"));

/** The Core def a Legacy def freezes, by its name after "Legacy: ". */
function coreOf(def: CardDef): CardDef {
  const found = Object.values(CATALOG).find((other) => other.set === "Core" && `Legacy: ${other.name}` === def.name);
  if (found === undefined) throw new Error(`no Core card for ${def.id}`);
  return found;
}

/** Every Legacy face beside its Core card's face, one pair per card. */
function Pairs({ radiant }: { radiant: boolean }) {
  return (
    <div style={{ display: "flex", flexWrap: "wrap", gap: 8 }}>
      {LEGACY.map((def) => (
        <div key={def.id} style={{ display: "flex", gap: 4 }}>
          {[{ id: def.id, def }, { id: coreOf(def).id, def: coreOf(def) }].map(({ id, def: faceDef }) => (
            <div
              key={id}
              data-face={id}
              style={{ width: FACE_WIDTH_PX, height: Math.round(FACE_WIDTH_PX / FACE_ASPECT), flex: "none" }}
            >
              <CardFace face={faceModel({ defId: id, def: faceDef, radiant })} layout="full" />
            </div>
          ))}
        </div>
      ))}
    </div>
  );
}

/** The painted art URI of the face with this id, or "" when it has none yet. */
function pictureOf(doc: Document, id: string): string {
  const art = doc.querySelector(`[data-face="${id}"] .cf-art`);
  if (art === null || !(art instanceof HTMLElement)) return "";
  return /^url\("(data:image\/svg\+xml,[^"]*)"\)$/.exec(getComputedStyle(art).backgroundImage)?.[1] ?? "";
}

/** A picture's pixels, sampled down to SAMPLE_PX square. */
function pixelsOf(uri: string): Promise<Uint8ClampedArray> {
  return new Promise((resolve, reject) => {
    const image = new Image();
    image.onload = (): void => {
      const canvas = document.createElement("canvas");
      canvas.width = SAMPLE_PX;
      canvas.height = SAMPLE_PX;
      const context = canvas.getContext("2d");
      if (context === null) {
        reject(new Error("no 2d context"));
        return;
      }
      context.drawImage(image, 0, 0, SAMPLE_PX, SAMPLE_PX);
      resolve(context.getImageData(0, 0, SAMPLE_PX, SAMPLE_PX).data);
    };
    image.onerror = (): void => {
      reject(new Error("the art did not load"));
    };
    image.src = uri;
  });
}

/** The share of pixels whose red, green or blue differ by more than CHANNEL_SLACK. */
function changedShare(a: Uint8ClampedArray, b: Uint8ClampedArray): number {
  let changed = 0;
  const pixels = Math.floor(a.length / 4);
  for (let i = 0; i < pixels; i += 1) {
    const base = i * 4;
    const dr = Math.abs((a[base] ?? 0) - (b[base] ?? 0));
    const dg = Math.abs((a[base + 1] ?? 0) - (b[base + 1] ?? 0));
    const db = Math.abs((a[base + 2] ?? 0) - (b[base + 2] ?? 0));
    if (dr > CHANNEL_SLACK || dg > CHANNEL_SLACK || db > CHANNEL_SLACK) changed += 1;
  }
  return pixels === 0 ? 0 : changed / pixels;
}

for (const radiant of [false, true]) {
  const face = radiant ? "radiant" : "base";
  it(`R2724000 every Legacy ${face} picture paints and differs from its Core card's, pixel by pixel`, () => {
    cy.viewport(1280, 1100);
    cy.mount(<Pairs radiant={radiant} />);
    cy.document({ timeout: SETTLE_TIMEOUT_MS }).should((doc) => {
      expect(LEGACY).to.have.length(20);
      for (const def of LEGACY) {
        expect(pictureOf(doc, def.id), `${def.id} paints`).not.to.equal("");
        expect(pictureOf(doc, coreOf(def).id), `${coreOf(def).id} paints`).not.to.equal("");
      }
    });
    cy.screenshot(`legacy-beside-core-${face}`, { capture: "viewport", overwrite: true });
    cy.document().then((doc) =>
      Promise.all(
        LEGACY.map((def) =>
          Promise.all([pixelsOf(pictureOf(doc, def.id)), pixelsOf(pictureOf(doc, coreOf(def).id))]).then(([a, b]) => ({
            id: def.id,
            share: changedShare(a, b),
          })),
        ),
      ),
    ).then((shares) => {
      for (const { id, share } of shares) expect(share, `${id} differs`).to.be.at.least(MIN_CHANGED_SHARE);
    });
  });
}
