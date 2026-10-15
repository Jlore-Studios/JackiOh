// R915010, #920: the card creator as a player uses it: building a card with the blocks, seeing its
// script in the code view, saving it to the account, the checks' and the server's errors shown where
// they belong, and a card written in code kept in code.

import { cleanup, fireEvent, render, screen, waitFor, within } from "@testing-library/react";
import { afterEach, describe, expect, it, vi } from "vitest";

import type { CustomCard } from "@jackioh/shared";

import { ApiRequestError } from "../net/api.ts";
import type { CustomCardInput } from "../net/customCards.ts";
import type { ScriptProblem } from "../wasm/index.ts";
import type { BuilderModel } from "./builder.ts";
import { Creator, type CreatorApi } from "./Creator.tsx";
import { creatorTestid } from "./testids.ts";

afterEach(cleanup);

const TOKEN = "token-1";

/** A fake account's store, as the server keeps it: ids minted at create, a version moved by every update. */
function fakeServer(start: CustomCard[] = [], problems: (source: string) => ScriptProblem[] = () => []) {
  let cards = [...start];
  let minted = 0;
  const api: CreatorApi = {
    list: vi.fn(() => Promise.resolve({ cards: [...cards], limits: { cards: 50 } })),
    create: vi.fn((_token: string, input: CustomCardInput) => {
      minted += 1;
      const card: CustomCard = {
        id: `custom-${String(minted).padStart(32, "0")}`,
        version: 1,
        frame: input.frame,
        source: input.source,
        ...(input.builder === undefined ? {} : { builder: input.builder }),
      };
      cards = [...cards, card];
      return Promise.resolve({ card });
    }),
    update: vi.fn((_token: string, id: string, version: number, input: CustomCardInput) => {
      const held = cards.find((card) => card.id === id);
      if (held === undefined) return Promise.reject(new ApiRequestError(404, { code: "not_found", message: "There is no such custom card." }));
      if (held.version !== version) {
        return Promise.reject(
          new ApiRequestError(409, { code: "conflict", message: "A newer save of this card exists.", details: { version: held.version } }),
        );
      }
      const card: CustomCard = { ...held, version: held.version + 1, frame: input.frame, source: input.source };
      cards = cards.map((old) => (old.id === id ? card : old));
      return Promise.resolve({ card });
    }),
    remove: vi.fn((_token: string, id: string) => {
      const had = cards.some((card) => card.id === id);
      cards = cards.filter((card) => card.id !== id);
      return Promise.resolve({ deleted: had });
    }),
    check: vi.fn((card: CustomCard) => Promise.resolve({ script: problems(card.source) })),
  };
  return { api, cards: () => cards, bump: (id: string) => (cards = cards.map((card) => (card.id === id ? { ...card, version: card.version + 1 } : card))) };
}

function select(testid: string, value: string, scope: HTMLElement = document.body): void {
  const [element] = within(scope).getAllByTestId(testid);
  if (element === undefined) throw new Error(`no ${testid}`);
  fireEvent.change(element, { target: { value } });
}

function type(testid: string, value: string): void {
  fireEvent.change(screen.getByTestId(testid), { target: { value } });
}

/** Builds the e2e spec's card through the blocks: "Cry: deal 2 damage to an enemy unit". */
async function buildStrike(): Promise<void> {
  type(creatorTestid.name, "Spark Lancer");
  select(creatorTestid.target, "an enemy unit");
  fireEvent.click(screen.getByTestId(creatorTestid.addHook));
  fireEvent.click(screen.getByTestId(creatorTestid.addStep));
  select(creatorTestid.stepAmount, "2");
  await waitFor(() => expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(false));
}

describe("R915010 the creator builds a card with blocks and saves it to the account", () => {
  it("R915010 the blocks write kit Luau, the code view shows it, and a save sends the frame, the script and the model", async () => {
    const server = fakeServer();
    const onTry = vi.fn();
    render(<Creator token={TOKEN} api={server.api} onTry={onTry} />);
    await screen.findByText("0 of 50 cards");

    await buildStrike();
    expect((screen.getByTestId(creatorTestid.text) as HTMLTextAreaElement).value).toBe("Cry: Deal {dmg} damage to an enemy unit.");

    fireEvent.click(screen.getByTestId(creatorTestid.viewCode));
    const code = (screen.getByTestId(creatorTestid.codeInput) as HTMLTextAreaElement).value;
    expect(code).toContain('target = "an enemy unit"');
    expect(code).toContain("c:damage(c:target(), n.dmg)");
    // The highlighted copy under the editor marks the kit's verb.
    expect(within(screen.getByTestId(creatorTestid.code)).getAllByText("damage")[0]?.className).toContain("creator-tok--kit");
    fireEvent.click(screen.getByTestId(creatorTestid.viewBuilder));
    expect(screen.getByTestId(creatorTestid.builder)).toBeTruthy();

    fireEvent.click(screen.getByTestId(creatorTestid.save));
    await screen.findByTestId(creatorTestid.saved);
    expect(server.api.create).toHaveBeenCalledTimes(1);
    const [, sent] = vi.mocked(server.api.create).mock.calls[0] ?? [];
    expect(sent?.frame).toMatchObject({ name: "Spark Lancer", type: "Unit", text: "Cry: Deal {dmg} damage to an enemy unit.", params: [{ key: "dmg", base: 2, radiant: 4 }] });
    expect(sent?.source).toBe(code);
    expect((sent?.builder as BuilderModel | undefined)?.target).toBe("an enemy unit");
    const listed = await screen.findAllByTestId(creatorTestid.listItem);
    expect(listed.map((item) => item.textContent)).toEqual([expect.stringContaining("Spark Lancer")]);
    expect(screen.getByText("1 of 50 cards")).toBeTruthy();

    // A second save is an update of the version it read.
    type(creatorTestid.cost, "2");
    await waitFor(() => expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId(creatorTestid.save));
    await waitFor(() => expect(server.api.update).toHaveBeenCalledTimes(1));
    expect(vi.mocked(server.api.update).mock.calls[0]?.[2]).toBe(1);

    fireEvent.click(screen.getByTestId(creatorTestid.tryIt));
    await waitFor(() => expect(onTry).toHaveBeenCalledWith(server.cards()[0]?.id));
  });

  it("R915010 a card saved on one device is listed on another, and opens in the builder there", async () => {
    const server = fakeServer();
    const first = render(<Creator token={TOKEN} api={server.api} onTry={vi.fn()} />);
    await screen.findByText("0 of 50 cards");
    await buildStrike();
    fireEvent.click(screen.getByTestId(creatorTestid.save));
    await screen.findByTestId(creatorTestid.saved);
    first.unmount();

    render(<Creator token={TOKEN} api={server.api} onTry={vi.fn()} />);
    const [item] = await screen.findAllByTestId(creatorTestid.listItem);
    if (item === undefined) throw new Error("the second device lists no card");
    fireEvent.click(item);
    expect((screen.getByTestId(creatorTestid.name) as HTMLInputElement).value).toBe("Spark Lancer");
    expect(screen.getByTestId(creatorTestid.builder)).toBeTruthy();
  });
});

describe("R915010 the creator shows the checks' and the server's errors where they belong", () => {
  it("R915010 a script problem is shown on its line in the code view, and the card is not saved while it stands", async () => {
    const server = fakeServer([], (source) => (source.includes("c:draw(1\n") ? [{ line: 3, message: "Expected ')' (to close '(' at column 11), got 'end'" }] : []));
    render(<Creator token={TOKEN} api={server.api} onTry={vi.fn()} />);
    await screen.findByText("0 of 50 cards");
    type(creatorTestid.name, "Broken");
    fireEvent.click(screen.getByTestId(creatorTestid.viewCode));
    type(creatorTestid.codeInput, "return Card {\n  cry = function(c, n)\n    c:draw(1\n  end,\n}\n");

    const problem = await screen.findByTestId(creatorTestid.scriptProblem);
    expect(problem.textContent).toContain("Line 3");
    expect(problem.textContent).toContain("Expected ')'");
    const marked = screen.getByTestId(creatorTestid.codeProblemLine);
    expect(marked.getAttribute("data-line")).toBe("3");
    expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(true);
    // A code edit beyond the model's script: the builder says why it cannot show the card.
    expect(screen.getByTestId(creatorTestid.builderRefusal).textContent).toMatch(/changed in the code view/u);
    expect((screen.getByTestId(creatorTestid.viewBuilder) as HTMLButtonElement).disabled).toBe(true);
  });

  it("R915010 a frame out of its limits shows the validator's issue under its field", async () => {
    const server = fakeServer();
    render(<Creator token={TOKEN} api={server.api} onTry={vi.fn()} />);
    await screen.findByText("0 of 50 cards");
    type(creatorTestid.name, "Too dear");
    type(creatorTestid.cost, "99");
    const issue = await screen.findByTestId(creatorTestid.frameIssue);
    expect(issue.getAttribute("data-field")).toBe("cost");
    expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(true);
  });

  it("R915010 the server's refusal is shown as it sent it: its issues under their fields, a stale version with the newer save to open", async () => {
    const server = fakeServer();
    vi.mocked(server.api.create).mockRejectedValueOnce(
      new ApiRequestError(400, {
        code: "bad_request",
        message: "This custom card breaks a limit.",
        details: { issues: [{ field: "name", message: "a name is 1 to 40 characters, not only spaces" }] },
      }),
    );
    render(<Creator token={TOKEN} api={server.api} onTry={vi.fn()} />);
    await screen.findByText("0 of 50 cards");
    await buildStrike();
    fireEvent.click(screen.getByTestId(creatorTestid.save));
    expect((await screen.findByTestId(creatorTestid.saveError)).textContent).toBe("This custom card breaks a limit.");
    const issue = screen.getByTestId(creatorTestid.frameIssue);
    expect(issue.getAttribute("data-field")).toBe("name");

    // Saved for real, then saved again elsewhere: this tab's save is stale.
    fireEvent.click(screen.getByTestId(creatorTestid.save));
    await screen.findByTestId(creatorTestid.saved);
    const id = server.cards()[0]?.id ?? "";
    server.bump(id);
    type(creatorTestid.cost, "3");
    await waitFor(() => expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(false));
    fireEvent.click(screen.getByTestId(creatorTestid.save));
    expect((await screen.findByTestId(creatorTestid.saveError)).textContent).toContain("version 2");
    fireEvent.click(screen.getByTestId(creatorTestid.reloadStale));
    await waitFor(() => expect((screen.getByTestId(creatorTestid.cost) as HTMLInputElement).value).toBe("1"));
  });
});

describe("R915010 a card written in code stays in code", () => {
  it("R915010 a saved card with no model opens in the code view, and the builder says why it cannot show it", async () => {
    const card: CustomCard = {
      id: "custom-0123456789abcdef0123456789abcdef",
      version: 1,
      frame: { name: "Hand made", type: "Spell", cost: 1, rarity: "Rare", text: "Draw a card.", params: [] },
      source: "return Card {\n  cry = function(c, n)\n    c:draw(1)\n  end,\n}\n",
    };
    const server = fakeServer([card]);
    render(<Creator token={TOKEN} api={server.api} onTry={vi.fn()} />);
    const [item] = await screen.findAllByTestId(creatorTestid.listItem);
    if (item === undefined) throw new Error("no card listed");
    fireEvent.click(item);
    expect((screen.getByTestId(creatorTestid.codeInput) as HTMLTextAreaElement).value).toBe(card.source);
    expect(screen.getByTestId(creatorTestid.builderRefusal).textContent).toMatch(/written in code/u);
    expect(screen.queryByTestId(creatorTestid.builder)).toBeNull();
    // Nothing changed, so nothing to save; it can be tried as it is.
    await waitFor(() => expect((screen.getByTestId(creatorTestid.tryIt) as HTMLButtonElement).disabled).toBe(false));
    expect((screen.getByTestId(creatorTestid.save) as HTMLButtonElement).disabled).toBe(true);

    fireEvent.click(screen.getByTestId(creatorTestid.delete));
    fireEvent.click(screen.getByTestId(creatorTestid.deleteConfirm));
    await waitFor(() => expect(server.cards()).toEqual([]));
  });
});
