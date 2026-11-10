// R3344000 (#3344): the picture a card face draws, however it is cut. A face with depth draws its picture as four planes
// (CardFace, art/pictures.ts `ART_PLANES`), each the painter's layers of its roles, so no one window holds the picture any
// more; a face without depth draws it whole in its one window. `facePicture` is the picture either way, so a test that
// asks whether two faces draw the same picture, or two different ones, asks it of a Common's one window and an Epic's
// four alike: a plane's layers are the whole picture's, each in one plane (depth.test.tsx's partition test), so the planes' layers
// together, in any order, are the whole's.
//
// R3350000 (#3350): a Mythic's diorama cuts its picture in slices (art/dioramas/), each a window of its own (`data-art-slice`:
// the base window, its planes', and a board face's `.cf-art-slice` layers), whose subject group may hold only some of the
// subject's elements and whose loop wraps its pieces in `<g class="dio">`. So `facePicture` and `wholePicture` compare
// flattened layers (`flatLayers`): each loop's wrapper replaced by what it holds, every element that is no group a layer of
// its own inside every group it stands in (each group's attributes kept), and a loop's style left out, its slices' pieces
// being the picture's whatever loop moves them. `layersOf` stays the picture's own top-level layers.

/** A picture's CSS background, decoded to the SVG document it holds (or what is there when it is not one). */
function decoded(art: HTMLElement | null | undefined): string {
  return decodeURIComponent(art?.style.backgroundImage ?? "");
}

/** The SVG of a decoded CSS background, between its first `<svg` and its last `</svg>`; null when there is none. */
function documentOf(background: string): string | null {
  const from = background.indexOf("<svg");
  const to = background.lastIndexOf("</svg>");
  return from < 0 || to < 0 ? null : background.slice(from, to + "</svg>".length);
}

/**
 * The layers of an SVG picture: the markup of each of its drawn elements, each its own string, in the order it draws
 * them, and its definitions and style apart.
 */
export function layersOf(svg: string): { defs: string; style: string; layers: string[] } {
  const root = new DOMParser().parseFromString(svg, "image/svg+xml").documentElement;
  const serializer = new XMLSerializer();
  const layers: string[] = [];
  let defs = "";
  let style = "";
  for (const child of Array.from(root.children)) {
    const markup = serializer.serializeToString(child);
    if (child.localName === "defs") defs = markup;
    else if (child.localName === "style") style = markup;
    else layers.push(markup);
  }
  return { defs, style, layers };
}

/**
 * The windows a face draws its picture in: its depth planes' pictures after its base window's, or the base window alone.
 * R3350000: a board face's diorama slices over its base window (`.cf-art-slice`) too.
 */
export function pictureWindows(cf: HTMLElement): HTMLElement[] {
  const base = cf.querySelector<HTMLElement>(".cf-art");
  if (base === null) return [];
  return [base, ...base.querySelectorAll<HTMLElement>(":scope > .cf-art-slice"), ...cf.querySelectorAll<HTMLElement>(".cf-planes .cf-art")];
}

/** The class a diorama slice's loop wraps its pieces in, and what its style's rule starts with (art/dioramas/index.ts). */
const DIORAMA_LOOP = "dio";
const DIORAMA_LOOP_RULE = `.${DIORAMA_LOOP}{`;

/**
 * Each element below `node` that is no group, written inside every group it stands in (each group's own tag and
 * attributes), one string a leaf, pushed to `out`; `open` is the groups' tags it stands in so far.
 */
function leaves(node: Element, serializer: XMLSerializer, open: string, close: string, out: string[]): void {
  if (node.localName !== "g") {
    out.push(`${open}${serializer.serializeToString(node)}${close}`);
    return;
  }
  const tag = serializer.serializeToString(node.cloneNode(false)).replace(/\/>$/, ">");
  for (const child of Array.from(node.children)) leaves(child, serializer, `${open}${tag}`, `</g>${close}`, out);
}

/**
 * R3350000: the layers of an SVG picture as `facePicture` compares them: with each diorama loop's wrapper replaced by what it
 * holds, every drawn element that is no group a layer of its own inside every group it stands in (their attributes kept),
 * and a loop's style left out; its definitions and its own style apart, as `layersOf` gives them.
 */
function flatLayers(svg: string): { defs: string; style: string; layers: string[] } {
  const root = new DOMParser().parseFromString(svg, "image/svg+xml").documentElement;
  // Only a diorama slice's loop has one (and jsdom's selector engine is slow on an SVG document).
  if (svg.includes(`class="${DIORAMA_LOOP}"`)) {
    for (const loop of Array.from(root.getElementsByTagName("g")).filter((g) => g.getAttribute("class") === DIORAMA_LOOP)) {
      loop.replaceWith(...Array.from(loop.childNodes));
    }
  }
  const serializer = new XMLSerializer();
  const layers: string[] = [];
  let defs = "";
  let style = "";
  for (const child of Array.from(root.children)) {
    if (child.localName === "defs") defs = serializer.serializeToString(child);
    else if (child.localName === "style") {
      if (!(child.textContent ?? "").includes(DIORAMA_LOOP_RULE)) style = serializer.serializeToString(child);
    } else leaves(child, serializer, "", "", layers);
  }
  return { defs, style, layers };
}

/**
 * The picture `cf` draws in its art window, decoded and written as `wholePicture` writes one: its definitions, its style
 * and every layer of it, sorted. A face with depth draws one picture in four windows, so its picture is the planes' layers
 * together (their one set of definitions and style, and every layer of every plane), which equals the same card's picture
 * drawn whole exactly when the planes hold every layer of it, once. A window that holds no SVG (a lazy face's gradient
 * before its picture draws) is returned as its CSS background reads.
 */
export function facePicture(cf: HTMLElement): string {
  const windows = pictureWindows(cf);
  const documents = windows.map((art) => documentOf(decoded(art)));
  if (windows.length === 0) return "";
  if (documents.some((svg) => svg === null)) return windows.map((art) => decoded(art)).join("");
  const planes = documents.map((svg) => flatLayers(svg ?? ""));
  return canonical(planes[0]?.defs ?? "", planes[0]?.style ?? "", planes.flatMap((plane) => plane.layers));
}

/** A picture's definitions, its style and its layers sorted, one string. */
function canonical(defs: string, style: string, layers: readonly string[]): string {
  return `${defs}${style}${[...layers].sort().join("")}`;
}

/** The same for a picture drawn whole, from its decoded CSS background (`proceduralArtUri`): `facePicture`'s form. */
export function wholePicture(background: string): string {
  const svg = documentOf(background);
  if (svg === null) return background;
  const { defs, style, layers } = flatLayers(svg);
  return canonical(defs, style, layers);
}
