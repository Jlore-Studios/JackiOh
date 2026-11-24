// Shared helpers: every frame is a pure function of t. BEAT is edm-2's (172 bpm, tracks.mjs edmDnb), from frame 0.
const BEAT = 60 / 172;
const B = (k) => k * BEAT;
const $ = (id) => document.getElementById(id);
const clamp = (x, a = 0, b = 1) => Math.max(a, Math.min(b, x));
const seg = (t, a, b) => clamp((t - a) / (b - a));
const eOut = (x) => 1 - Math.pow(1 - x, 3);
const eIn = (x) => x * x * x;
const eIO = (x) => (x < 0.5 ? 4 * x * x * x : 1 - Math.pow(-2 * x + 2, 3) / 2);
const eBack = (x) => { const c1 = 1.70158, c3 = c1 + 1; return 1 + c3 * Math.pow(x - 1, 3) + c1 * Math.pow(x - 1, 2); };
const lerp = (a, b, x) => a + (b - a) * x;
const show = (id, on) => { $(id).style.display = on ? "block" : "none"; };
const pending = [];
function setSrc(img, src) { if (img.getAttribute("src") !== src) img.setAttribute("src", src); pending.push(img); }
const pad = (n, w) => String(n).padStart(w, "0");
const still = (tag, i) => `stills/${tag}/${pad(i, 3)}.png`;
const seqSrc = (tag, page) => `seq/${tag}/${pad(Math.max(0, Math.floor(page * 60 + 1e-6)), 4)}.jpg`;
// Text in: fade and rise over d seconds from at; a slam: scale down from big onto the line.
function textIn(el, t, at, d = 0.16, rise = 18) { const x = eOut(seg(t, at, at + d)); el.style.opacity = x; el.style.transform = `translateY(${(1 - x) * rise}px)`; }
function slam(el, t, at, d = 0.12, from = 1.6) { const x = seg(t, at, at + d); el.style.opacity = clamp(x * 3); el.style.transform = `scale(${lerp(from, 1, eOut(x))})`; }
function hide(...els) { for (const e of els) e.style.opacity = 0; }
// The montage: stills of the real game, one per half beat.
const MONTAGE = [0, 1, 3, 4, 6, 7, 9, 10, 12, 14, 16, 17, 19, 21, 24, 27];
// p1's turn starts: mana 1/1, 2/2, 3/3, 4/4 (stills/*/NNN.json).
const MANA = [0, 5, 12, 23];
// Deck A, the 25 cards that played this game.
const DECK = ["core-008","traininggrounds-003","traininggrounds-006","traininggrounds-032","core-015","meditative-084",
  "meditative-100","core-020","core-045","meditative-002","meditative-073","traininggrounds-005","core-012",
  "traininggrounds-008","meditative-003","traininggrounds-023","traininggrounds-033","traininggrounds-009",
  "meditative-094","core-041","meditative-064","core-005","core-077","traininggrounds-016","meditative-093"];
async function settleImages() {
  await Promise.all(pending.map((img) => (img.complete && img.naturalWidth ? Promise.resolve() : new Promise((r) => { img.onload = img.onerror = () => r(); }))));
  await Promise.all(pending.map((img) => img.decode().catch(() => {})));
}
function buildGrid(box, cols, fw, fh, gap) {
  const out = [];
  DECK.forEach((id, i) => {
    const f = document.createElement("div"); f.className = "face";
    const c = i % cols, r = Math.floor(i / cols);
    f.style.left = `${c * (fw + gap)}px`; f.style.top = `${r * (fh + gap)}px`; f.style.width = `${fw}px`; f.style.height = `${fh}px`;
    const img = document.createElement("img"); img.src = `faces/${id}-base.png`; f.appendChild(img); box.appendChild(f); out.push(f);
  });
  return out;
}
