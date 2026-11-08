// R3344000 (#3344): the picture a card face draws, however it is cut. A face with depth draws its picture as four planes
// (CardFace, art/pictures.ts `ART_PLANES`), each the painter's layers of its roles, so no one window holds the picture any
// more; a face without depth draws it whole in its one window. `facePicture` is the picture either way, so a test that
// asks whether two faces draw the same picture, or two different ones, asks it of a Common's one window and an Epic's
// four alike: a plane's layers are the whole picture's, each in one plane (depth.test.tsx's partition test), so the planes' layers
// together, in any order, are the whole's.

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

/** The windows a face draws its picture in: its depth planes' pictures after its base window's, or the base window alone. */
export function pictureWindows(cf: HTMLElement): HTMLElement[] {
  const base = cf.querySelector<HTMLElement>(".cf-art");
  if (base === null) return [];
  return [base, ...cf.querySelectorAll<HTMLElement>(".cf-planes .cf-art")];
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
  const planes = documents.map((svg) => layersOf(svg ?? ""));
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
  const { defs, style, layers } = layersOf(svg);
  return canonical(defs, style, layers);
}
