// R2784000 #2792: `/dev/fx`, the motif gallery: it offers every drawn motif, every flavor's
// palette and sub-look, every gallery scale and every card whose table entry names a motif, and
// each plays something; #2793: every Core entry other than `none` plays as its card's play, #2796 every
// Meditative and Meditative+ one, R2795000 every Classic+ one, a Field card wearing its aura, #2798
// every Stinkyscape part-two one, #2799 every Training Grounds one, R2794000 every Classic one, a
// marquee Unit's entrance as it lands, and R2797000 a Death, trigger or Activate from its hook's marker.
// R2785000: it plays a card as the fixture set piece, which holds the board until a click skips it.
// R3332000: it lists every signature by set; R3332001 plays Felinor Fiender's over its Felinors.
// R3400001: Meditative+'s and Training Grounds' eight signatures, each tile aura standing as its Unit lands.

import { afterEach, describe, expect, it, vi } from "vitest";
import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";

import { PLAY_IMPACT_OPPONENT_GAIN } from "../../audio/constants.ts";
import { setAudioEngineForTests } from "../../audio/engine.ts";
import { fakeAudioEngine } from "../../audio/test/fakeAudio.ts";
import { planAuras } from "../../fx/aura/plan.ts";
import { entranceKeyOf, planCardFx, planCardMotif } from "../../fx/cardFx.ts";
import { FX_CARD_FILES, FX_CARD_TABLE, parseFxCardTable } from "../../fx/cards/tables.ts";
import { FX_GALLERY_SCALES, FX_INTENSITY_SCALE } from "../../fx/constants.ts";
import { planFx } from "../../fx/cues.ts";
import { FLAVOR_SUB_LOOKS, FX_PALETTES } from "../../fx/flavors.ts";
import { createFxMemory } from "../../fx/memory.ts";
import {
  GALLERY_SET_PIECE,
  SYSTEM_MOMENTS,
  galleryBoardOf,
  galleryCardEvents,
  galleryCues,
  galleryEntry,
  galleryFieldView,
  galleryPlayOf,
  systemMomentFixture,
} from "../../fx/motifs/gallery.ts";
import { hasSetPiece, signatureRunOf } from "../../fx/setPieces.ts";
import type { FxPlanEnv } from "../../fx/types.ts";
import { BURST_BUDGET_MS, planEntries } from "../../game/animations.ts";
import { PLAY_TIERS_ASCENDING } from "../../game/damageFeel.ts";
import { FX_DRAWN_MOTIFS } from "../../fx/motifs/names.ts";
import { fullBoardView, withEvents } from "../../test/fixtures.ts";
import { CATALOG } from "../../wire/cards.ts";
import FxGallery, { fxGalleryTestid, galleryCards, gallerySignatures } from "./fx.tsx";

afterEach(cleanup);

function optionsOf(testid: string): string[] {
  return [...screen.getByTestId(testid).querySelectorAll("option")].map((option) => option.value);
}

describe("R2784000 #2792 the /dev/fx gallery and sub-look", () => {
  it("R2784000 #2792 the gallery lists every drawn motif, palette and sub-look, scale and example", () => {
    render(<FxGallery />);
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
    expect(optionsOf(fxGalleryTestid.motif)).toEqual([...FX_DRAWN_MOTIFS]);
    expect(optionsOf(fxGalleryTestid.palette)).toEqual([...FX_PALETTES]);
    const subs = Object.values(FLAVOR_SUB_LOOKS).reduce((n, looks) => n + Object.keys(looks ?? {}).length, 0);
    expect(optionsOf(fxGalleryTestid.palette)).toHaveLength(15 + subs);
    expect(optionsOf(fxGalleryTestid.palette)).toContain("Tech/rocket");
    expect(optionsOf(fxGalleryTestid.scale)).toEqual(FX_GALLERY_SCALES.map(String));
    const cards = optionsOf(fxGalleryTestid.card);
    expect(cards).toEqual(galleryCards().map((card) => card.defId));
    // #2793: Core's 92 entries other than `none` (76 on the play, 7 on a hook, 5 recipes, 3 set pieces,
    // and R3332001 Felinor Fiender's entrance);
    // #2796: Meditative's 116 and Meditative+'s 29; R2795000: Classic+'s 96 (74 cards and 22 tokens);
    // #2798: Stinkyscape part two's 88 (R2801000: its six Starship cards among them; R3336000: the Propagation
    // layer's activate is in fx/battleship.ts, leaving its table entry none); R2797000: Stinkyscape
    // 001 to 090's 94; R2794000: Classic's 79 (its plays' motifs, its hooks' on their markers, its three
    // Flourishes, Plague Nuke's set piece and its two marquee entrances); #2799: Training Grounds' 51.
    // R3338000: Event Horizon's and The Big Crunch's set pieces, which were `none`; R3334004: the Dormant
    // Starship's, Starfleet Commander's and Preemptive Strike's signatures, which were `none`. R3400001: the
    // Training-Grounds Golem's signature, which was `none`.
    expect(cards).toHaveLength(651);
    expect(cards).toContain("classic-071");
    // Playing a motif and a card's entry draws without throwing, whatever is picked.
    fireEvent.change(screen.getByTestId(fxGalleryTestid.motif), { target: { value: "lance" } });
    fireEvent.change(screen.getByTestId(fxGalleryTestid.palette), { target: { value: "Tech/rocket" } });
    fireEvent.change(screen.getByTestId(fxGalleryTestid.scale), { target: { value: "2" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.play));
    fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: "classic-071" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
  });

  it("R2794000 the gallery has a play for every Classic entry other than none, and each plays its motif, Flourish, entrance or set piece", () => {
    const view = fullBoardView();
    const intensity = FX_INTENSITY_SCALE.normal;
    const classic = [...FX_CARD_TABLE].filter(([defId, entry]) => defId.startsWith("classic-") && entry.kind !== "none");
    expect(classic).toHaveLength(79);
    expect(galleryCards().filter((card) => card.defId.startsWith("classic-")).map((card) => card.defId)).toEqual(classic.map(([defId]) => defId));
    for (const [defId, entry] of classic) {
      const play = galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      // Planned as the board's effects layer plans it: remember, then plan.
      const events = galleryCardEvents(defId, play, view, 1);
      const env: FxPlanEnv = { intensity, card: () => undefined, memory: createFxMemory() };
      const entries = planEntries(events, withEvents(galleryFieldView(defId, view), events), false, undefined, hasSetPiece, signatureRunOf);
      let drawn = false;
      // A marquee Unit's entrance plays as it lands, Nature Titan's before its Cry's motif.
      let landed = entranceKeyOf(defId) === undefined || play.kind === "setPiece";
      for (const planned of entries) {
        env.memory.remember(planned.events);
        const recipe = planned.events.some((event) => planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs }) !== null);
        // R2797000: a hook's motif draws from its marker under the play, as the board draws it.
        if ((play.kind === "board" || play.kind === "hook") && planCardMotif(planned, planned.view, env) !== null) drawn = true;
        if ((play.kind === "recipe" || play.kind === "entrance") && recipe) drawn = true;
        // R3338000: a Terminus card's set piece is its signature's, a board clear's its wipe's.
        if (play.kind === "setPiece" && (planned.wipe?.by === defId || planned.signature?.by === defId)) drawn = true;
        if (recipe && planned.events.some((event) => event.type === "summoned")) landed = true;
      }
      expect(drawn, `${defId} ${play.kind}`).toBe(true);
      expect(landed, `${defId} entrance`).toBe(true);
    }
  });

  it("R2784000 #2792 every motif plays in every palette and sub-look at every scale on the fixture board", () => {
    const view = fullBoardView();
    for (const motif of FX_DRAWN_MOTIFS) {
      for (const palette of FX_PALETTES) {
        for (const scale of FX_GALLERY_SCALES) {
          expect(galleryCues(motif, palette, scale, view, FX_INTENSITY_SCALE.normal).length, `${motif} ${palette} ×${String(scale)}`).toBeGreaterThan(0);
        }
      }
      // The effects off plays nothing.
      expect(galleryCues(motif, "Fire", 1, view, FX_INTENSITY_SCALE.off)).toEqual([]);
    }
  });

  it("R2784000 the gallery lists and plays every system moment", () => {
    render(<FxGallery />);
    expect(optionsOf(fxGalleryTestid.moment)).toEqual([...SYSTEM_MOMENTS]);
    const base = fullBoardView();
    for (const moment of SYSTEM_MOMENTS) {
      const events = systemMomentFixture(moment, base);
      expect(events.length, `${moment} has a fixture`).toBeGreaterThan(0);
      const entry = galleryEntry(events, base);
      const memory = createFxMemory();
      memory.remember(entry.events);
      expect(planFx(entry, base, { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory }).length, `${moment} plans`).toBeGreaterThan(0);
    }
    fireEvent.change(screen.getByTestId(fxGalleryTestid.moment), { target: { value: "glitched" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playMoment));
    expect(screen.getByTestId(fxGalleryTestid.root)).toBeInTheDocument();
  });

  /**
   * Every entry of `ids` other than none has a gallery play, in table order, and each plays its motif, recipe
   * or set piece as the board's effects layer plans it; a Field card's aura stands where the gallery lays it.
   */
  function expectGalleryPlays(ids: ReadonlySet<string>, count: number): void {
    const view = fullBoardView();
    const intensity = FX_INTENSITY_SCALE.normal;
    const drawable = [...FX_CARD_TABLE].filter(([defId, entry]) => ids.has(defId) && entry.kind !== "none");
    expect(drawable).toHaveLength(count);
    expect(galleryCards().filter((card) => ids.has(card.defId)).map((card) => card.defId)).toEqual(drawable.map(([defId]) => defId));
    for (const [defId, entry] of drawable) {
      // R2787000: a Field card's aura stands in the backrow zone the gallery lays it in.
      if (entry.kind === "hooks" && entry.aura !== undefined && galleryFieldView(defId, view) !== view) {
        expect(planAuras(galleryFieldView(defId, view), new Map()).map((aura) => aura.defId), defId).toContain(defId);
      }
      const play = galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      if (play.kind === "aura") continue;
      // Planned as the board's effects layer plans it: remember, then plan, over the board its entrance reads (R3332001).
      const board = galleryBoardOf(defId, view);
      const events = galleryCardEvents(defId, play, board, 1);
      const env: FxPlanEnv = { intensity, card: () => undefined, memory: createFxMemory() };
      const entries = planEntries(events, withEvents(galleryFieldView(defId, board), events), false, undefined, hasSetPiece, signatureRunOf);
      let drawn = false;
      for (const planned of entries) {
        env.memory.remember(planned.events);
        // R2797000: a hook's motif draws from its marker under the play, as the board draws it.
        if ((play.kind === "board" || play.kind === "hook") && planCardMotif(planned, planned.view, env) !== null) drawn = true;
        // R3332000: a recipe, and an entrance alone as its card lands.
        const recipe = planned.events.some((event) => planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs }) !== null);
        if ((play.kind === "recipe" || play.kind === "entrance") && recipe) drawn = true;
        // R3338000: a Terminus card's set piece is its signature's, a board clear's its wipe's.
        if (play.kind === "setPiece" && (planned.wipe?.by === defId || planned.signature?.by === defId)) drawn = true;
      }
      expect(drawn, `${defId} ${play.kind}`).toBe(true);
    }
  }

  it("R2784000 #2793 the gallery has a play for every Core entry other than none, and each plays its motif, recipe or set piece", () => {
    expectGalleryPlays(new Set(Object.keys(CATALOG).filter((id) => CATALOG[id]?.set === "Core")), 92);
  });

  it("R3332000 R3332001 the gallery lists every signature by set and plays Felinor Fiender's over its Felinors", () => {
    // Every signature of the table, grouped by its set in the tables' order; Core's Felinor Fiender the first.
    const signatures = gallerySignatures();
    const signed = [...FX_CARD_TABLE].flatMap(([defId, entry]) => (entry.kind !== "none" && entry.signature === true ? [defId] : []));
    expect(signatures.flatMap((group) => group.cards.map((card) => card.defId))).toEqual(signed);
    expect(signatures.map((group) => group.set)).toEqual([...new Set(signed.map((defId) => CATALOG[defId]?.set))]);
    for (const group of signatures) for (const card of group.cards) expect(CATALOG[card.defId]?.set, card.defId).toBe(group.set);
    expect(signatures[0]).toMatchObject({ set: "Core", cards: [{ defId: "core-092", play: { kind: "entrance", key: "felinorAssembly" } }] });

    // Planned over its board, Felinor Fiender's landing draws a streak from each of the three Felinors.
    const base = fullBoardView();
    const board = galleryBoardOf("core-092", base);
    expect(board).not.toBe(base);
    expect(galleryBoardOf("core-002", base)).toBe(base);
    const events = galleryCardEvents("core-092", { kind: "entrance", key: "felinorAssembly" }, board, 1);
    const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
    const shots = planEntries(events, withEvents(board, events), false).flatMap((planned) => {
      env.memory.remember(planned.events);
      return planFx(planned, planned.view, env).filter((cue) => cue.kind === "shot");
    });
    expect(shots.map((cue) => (cue.kind === "shot" && cue.from.kind === "testid" ? cue.from.testid : ""))).toEqual(["zone-you-units-1", "zone-you-units-2", "zone-you-units-5"]);

    // The rendered gallery offers it under its set, and its play puts the Felinors on the board first.
    render(<FxGallery />);
    const select = screen.getByTestId(fxGalleryTestid.signature);
    expect([...select.querySelectorAll("optgroup")].map((group) => group.getAttribute("label"))).toEqual(signatures.map((group) => group.set));
    expect(optionsOf(fxGalleryTestid.signature)).toEqual(signed);
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playSignature));
    for (const [lane, defId] of [[1, "core-t-felinor"], [2, "core-043"], [5, "core-012"]] as const) {
      expect(document.querySelector(`[data-testid="zone-you-units-${String(lane)}"] [data-def-id="${defId}"]`), `lane ${String(lane)}`).not.toBeNull();
    }
    expect(screen.getByTestId("game").querySelector("[data-animating]")).not.toBeNull();
  });

  it("R3400001 the gallery lists the eight signatures by set and stands each tile aura", () => {
    const signatures = gallerySignatures();
    const of = (set: string): string[] => signatures.find((group) => group.set === set)?.cards.map((card) => card.defId) ?? [];
    expect(of("Meditative+")).toEqual(["meditativeplus-001", "meditativeplus-024", "meditativeplus-026"]);
    expect(of("Training Grounds")).toEqual(["traininggrounds-037", "traininggrounds-038", "traininggrounds-039", "traininggrounds-039-1", "traininggrounds-040"]);
    const base = fullBoardView();
    for (const defId of [...of("Meditative+"), ...of("Training Grounds")]) {
      const entry = FX_CARD_TABLE.get(defId);
      const play = entry === undefined ? null : galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      expect(play.kind, defId).toBe(defId === "traininggrounds-039-1" ? "entrance" : "recipe");
      // Its play draws, every beat a shape or a card back, naming no card (R202).
      const board = galleryBoardOf(defId, base);
      const events = galleryCardEvents(defId, play, board, 1);
      const env: FxPlanEnv = { intensity: FX_INTENSITY_SCALE.normal, card: () => undefined, memory: createFxMemory() };
      const cues = planEntries(events, withEvents(galleryFieldView(defId, board), events), false).flatMap((planned) => {
        env.memory.remember(planned.events);
        return planFx(planned, planned.view, env);
      });
      expect(cues.length, defId).toBeGreaterThan(0);
      expect(JSON.stringify(cues), defId).not.toContain(defId);
      // A Unit's tile aura stands on its own unit zone, where its gallery entrance lands.
      const aura = entry !== undefined && entry.kind !== "none" && "aura" in entry ? entry.aura : undefined;
      if (aura?.scope !== "tile") continue;
      const fielded = galleryFieldView(defId, board);
      expect(fielded.you.units[2]?.defId, defId).toBe(defId);
      expect(fielded.you.units[2]?.instanceId, defId).toBe(base.you.units[2]?.instanceId);
      expect(planAuras(fielded, new Map()).filter((spec) => spec.defId === defId).map((spec) => [spec.side, spec.lane, spec.scope, spec.look]), defId).toEqual([["you", 3, "tile", aura.look]]);
    }
    expect(of("Meditative+").concat(of("Training Grounds")).filter((defId) => {
      const entry = FX_CARD_TABLE.get(defId);
      return entry !== undefined && entry.kind !== "none" && "aura" in entry && entry.aura?.scope === "tile";
    })).toEqual(["meditativeplus-024", "traininggrounds-038", "traininggrounds-039-1"]);

    // The rendered gallery offers each under its set, and plays Bob's.
    render(<FxGallery />);
    const select = screen.getByTestId(fxGalleryTestid.signature);
    for (const set of ["Meditative+", "Training Grounds"]) expect([...select.querySelectorAll("optgroup")].map((group) => group.getAttribute("label"))).toContain(set);
    fireEvent.change(select, { target: { value: "meditativeplus-024" } });
    fireEvent.click(screen.getByTestId(fxGalleryTestid.playSignature));
    expect(screen.getByTestId("game").querySelector("[data-animating]")).not.toBeNull();
  });

  it("R2784000 #2798 the gallery has a play for every Stinkyscape part-two entry other than none, and each plays its motif, recipe, set piece or aura", () => {
    const file = FX_CARD_FILES.find((candidate) => candidate.file === "stinkyscape-2.json5");
    if (file === undefined) throw new Error("no stinkyscape-2.json5");
    // R2801000: the pass's 83 and the Starship part's six; R3336000: Propagation layer is none; R3334004: the
    // Starship's three signatures.
    expectGalleryPlays(new Set(parseFxCardTable(file.file, file.source).keys()), 91);
  });

  it("R2784000 #2799 the gallery has a play for every Training Grounds entry other than none, and each plays its motif or recipe", () => {
    const file = FX_CARD_FILES.find((candidate) => candidate.file === "traininggrounds.json5");
    if (file === undefined) throw new Error("no traininggrounds.json5");
    // R3400001: the Training-Grounds Golem's signature joins them.
    expectGalleryPlays(new Set(parseFxCardTable(file.file, file.source).keys()), 52);
  });

  it("R2784000 #2796 the gallery plays every Meditative and Meditative+ entry", () => {
    const view = fullBoardView();
    const intensity = FX_INTENSITY_SCALE.normal;
    const sets = [...FX_CARD_TABLE].filter(([defId, entry]) => defId.startsWith("meditative") && entry.kind !== "none");
    expect(sets).toHaveLength(116 + 29);
    expect(galleryCards().filter((card) => card.defId.startsWith("meditative")).map((card) => card.defId)).toEqual(sets.map(([defId]) => defId));
    for (const [defId, entry] of sets) {
      const play = galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      // A Field card's aura stands in the first backrow zone while it plays.
      if (entry.kind === "hooks" && entry.aura !== undefined) {
        expect(planAuras(galleryFieldView(defId, view), new Map()).map((aura) => aura.defId), defId).toContain(defId);
      }
      if (play.kind === "aura") continue;
      // Planned as the board's effects layer plans it: remember, then plan.
      const events = galleryCardEvents(defId, play, view, 1);
      const env: FxPlanEnv = { intensity, card: () => undefined, memory: createFxMemory() };
      let drawn = false;
      for (const planned of planEntries(events, withEvents(galleryFieldView(defId, view), events), false, undefined, hasSetPiece, signatureRunOf)) {
        env.memory.remember(planned.events);
        // R2797000: a hook's motif draws from its marker under the play, as the board draws it.
        if ((play.kind === "board" || play.kind === "hook") && planCardMotif(planned, planned.view, env) !== null) drawn = true;
        if (play.kind === "recipe" && planned.events.some((event) => planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs }) !== null)) drawn = true;
      }
      expect(drawn, `${defId} ${play.kind}`).toBe(true);
    }
  });

  it("R2795000 the gallery has a play for every Classic+ entry other than none, and each plays its motif, recipe or set piece", () => {
    render(<FxGallery />);
    const offered = optionsOf(fxGalleryTestid.card);
    const view = fullBoardView();
    const intensity = FX_INTENSITY_SCALE.normal;
    const given = [...FX_CARD_TABLE].filter(([defId, entry]) => defId.startsWith("classicplus-") && entry.kind !== "none");
    expect(galleryCards().filter((card) => card.defId.startsWith("classicplus-")).map((card) => card.defId)).toEqual(given.map(([defId]) => defId));
    for (const [defId, entry] of given) {
      expect(offered, defId).toContain(defId);
      const play = galleryPlayOf(defId, entry);
      if (play === null) throw new Error(`${defId} has no gallery play`);
      // Planned as the board's effects layer plans it: remember, then plan.
      const events = galleryCardEvents(defId, play, view, 1);
      const env: FxPlanEnv = { intensity, card: () => undefined, memory: createFxMemory() };
      let drawn = false;
      for (const planned of planEntries(events, withEvents(galleryFieldView(defId, view), events), false, undefined, hasSetPiece, signatureRunOf)) {
        env.memory.remember(planned.events);
        // R2797000: a hook's motif draws from its marker under the play, as the board draws it.
        if ((play.kind === "board" || play.kind === "hook") && planCardMotif(planned, planned.view, env) !== null) drawn = true;
        if (play.kind === "recipe" && planned.events.some((event) => planCardFx(event, { entry: planned, view: planned.view, env, D: planned.durationMs }) !== null)) drawn = true;
        // R3338000: a Terminus card's set piece is its signature's, a board clear's its wipe's.
        if (play.kind === "setPiece" && (planned.wipe?.by === defId || planned.signature?.by === defId)) drawn = true;
      }
      expect(drawn, `${defId} ${play.kind}`).toBe(true);
    }
  });

  it("R2795000 a Classic+ Field card's gallery view stands it face-up, wearing its aura", () => {
    const view = fullBoardView();
    const isField = (defId: string): boolean => CATALOG[defId]?.type === "Field Spell" || CATALOG[defId]?.type === "Field Trap";
    const fields = galleryCards().filter((card) => card.defId.startsWith("classicplus-") && isField(card.defId));
    expect(fields.length).toBeGreaterThanOrEqual(10);
    const wears = (shown: typeof view, defId: string): boolean => planAuras(shown, new Map()).some((aura) => aura.defId === defId);
    for (const { defId } of fields) {
      const shown = galleryFieldView(defId, view);
      expect(shown.you.backrow[0], defId).toEqual(expect.objectContaining({ defId, faceDown: false }));
      expect(wears(view, defId), defId).toBe(false);
      expect(wears(shown, defId), defId).toBe(true);
    }
    // Any other card leaves the board as it stands.
    expect(galleryFieldView("classicplus-025", view)).toBe(view);
  });

  it("R2784000 #2793 the gallery plays a card of each kind, a Field Spell standing in the backrow with its aura", () => {
    vi.useFakeTimers();
    // jsdom lays nothing out: give every zone a box, so the aura layer measures the Field Spell's side.
    const box = vi.spyOn(HTMLElement.prototype, "getBoundingClientRect").mockReturnValue(new DOMRect(0, 0, 100, 100));
    try {
      render(<FxGallery />);
      // A cry motif, a trigger from its marker, a recipe, a set piece and a Field Spell's cast and aura.
      for (const defId of ["core-002", "core-013", "core-083", "core-017", "core-014"]) {
        fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: defId } });
        fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
        act(() => {
          vi.advanceTimersByTime(BURST_BUDGET_MS * 4);
        });
      }
      expect(document.querySelector('[data-aura-look="storm"]')).not.toBeNull();
    } finally {
      box.mockRestore();
      vi.useRealTimers();
    }
  });

  it("R2800000 the gallery plays Twisting Nether as its own set piece", () => {
    vi.useFakeTimers();
    try {
      render(<FxGallery />);
      const game = screen.getByTestId("game");
      fireEvent.change(screen.getByTestId(fxGalleryTestid.card), { target: { value: "core-088" } });
      fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
      // The play's own entry, then its wipe, which holds the board as the Nether.
      const inFlight = (): string | null => screen.queryByTestId("animation-queue")?.getAttribute("data-animating") ?? null;
      for (let waited = 0; waited < BURST_BUDGET_MS && inFlight() !== "destroyed"; waited += 10) {
        act(() => {
          vi.advanceTimersByTime(10);
        });
      }
      expect(game).toHaveAttribute("data-set-piece", "nether");
    } finally {
      vi.useRealTimers();
    }
  });

  it("R2785000 the gallery plays a card as the fixture set piece, which holds the board until a click skips it", () => {
    vi.useFakeTimers();
    try {
      render(<FxGallery />);
      const game = screen.getByTestId("game");
      fireEvent.click(screen.getByTestId(fxGalleryTestid.playSetPiece));
      expect(game).toHaveAttribute("data-set-piece", GALLERY_SET_PIECE);
      expect(screen.getByTestId("set-piece-skip")).toHaveTextContent("Tap to skip");
      fireEvent.click(game);
      expect(game).not.toHaveAttribute("data-set-piece");
      expect(screen.queryByTestId("set-piece-skip")).toBeNull();

      // Once the board has caught up, a plain play holds nothing.
      act(() => {
        vi.advanceTimersByTime(BURST_BUDGET_MS * 4);
      });
      fireEvent.click(screen.getByTestId(fxGalleryTestid.playCard));
      expect(game.querySelector("[data-animating]")).not.toBeNull();
      expect(game).not.toHaveAttribute("data-set-piece");
    } finally {
      vi.useRealTimers();
    }
  });
});

describe("R3346000 the gallery plays every play impact", () => {
  const TYPES = ["unit", "spell", "field"] as const;
  const cellOf = (type: string, tier: string): HTMLElement => {
    const cell = screen.getAllByTestId(fxGalleryTestid.impact).find((el) => el.dataset.type === type && el.dataset.tier === tier);
    if (cell === undefined) throw new Error(`no ${tier} ${type} cell`);
    return cell;
  };
  const rigged = (): ReturnType<typeof vi.fn> => {
    const playSfx = vi.fn(() => true);
    setAudioEngineForTests(fakeAudioEngine({ playSfx }));
    return playSfx;
  };
  afterEach(() => {
    setAudioEngineForTests(null);
  });

  it("R3346000 a cell for every type at every tier, each playing its impact as the viewer's own", () => {
    const playSfx = rigged();
    render(<FxGallery />);
    expect(screen.getAllByTestId(fxGalleryTestid.impact)).toHaveLength(TYPES.length * PLAY_TIERS_ASCENDING.length);
    for (const type of TYPES) {
      for (const tier of PLAY_TIERS_ASCENDING) {
        fireEvent.click(cellOf(type, tier));
        expect(playSfx, `${tier} ${type}`).toHaveBeenLastCalledWith("playImpact", { playTier: tier, playType: type }, 0);
      }
    }
    // Every press also clicks (the app's uiClick); the impacts are the rest.
    expect(playSfx.mock.calls.filter(([id]) => id === "playImpact")).toHaveLength(TYPES.length * PLAY_TIERS_ASCENDING.length);
  });

  it("R3346000 as the opponent every impact carries the opponent's gain, and unticked drops it again", () => {
    const playSfx = rigged();
    render(<FxGallery />);
    fireEvent.click(screen.getByTestId(fxGalleryTestid.impactOpponent));
    for (const type of TYPES) {
      for (const tier of PLAY_TIERS_ASCENDING) {
        fireEvent.click(cellOf(type, tier));
        expect(playSfx, `${tier} ${type}`).toHaveBeenLastCalledWith("playImpact", { playTier: tier, playType: type, gain: PLAY_IMPACT_OPPONENT_GAIN }, 0);
      }
    }
    fireEvent.click(screen.getByTestId(fxGalleryTestid.impactOpponent));
    fireEvent.click(cellOf("unit", "massive"));
    expect(playSfx).toHaveBeenLastCalledWith("playImpact", { playTier: "massive", playType: "unit" }, 0);
  });

  it("R3346000 the trap's set click and snap play, the snap a step quieter as the opponent's", () => {
    const playSfx = rigged();
    render(<FxGallery />);
    fireEvent.click(screen.getByTestId(fxGalleryTestid.trapSet));
    expect(playSfx).toHaveBeenLastCalledWith("trapSet", undefined, 0);
    fireEvent.click(screen.getByTestId(fxGalleryTestid.trapSnap));
    expect(playSfx).toHaveBeenLastCalledWith("trapSnap", undefined, 0);
    fireEvent.click(screen.getByTestId(fxGalleryTestid.impactOpponent));
    fireEvent.click(screen.getByTestId(fxGalleryTestid.trapSnap));
    expect(playSfx).toHaveBeenLastCalledWith("trapSnap", { gain: PLAY_IMPACT_OPPONENT_GAIN }, 0);
    fireEvent.click(screen.getByTestId(fxGalleryTestid.trapSet));
    expect(playSfx).toHaveBeenLastCalledWith("trapSet", undefined, 0);
  });
});
