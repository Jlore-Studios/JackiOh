// The dashboard from canned answers: each section shows what the server sent, and each button sends
// its request as typed; the status line shows the answer, or the server's refusal word for word.

import { render, screen, waitFor, within } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { describe, expect, it, vi } from "vitest";

import { adminApi } from "./api.ts";
import { Dashboard } from "./Dashboard.tsx";
import { NO_REWARD_NOTE } from "./News.tsx";
import { json, when } from "./section.ts";
import { AT, AUDIT, CONSTRUCTED, dashboardRoutes, ME, NEWS, QUESTS } from "./test/canned.ts";
import { fakeFetch, type Route } from "./test/fakes.ts";
import { testids } from "./testids.ts";
import { BARS, CLEAR_PARTS } from "./types.ts";
import { SHIPPED_SETS } from "./wire.ts";

function open(routes: Record<string, Route> = {}) {
  const fake = fakeFetch({ ...dashboardRoutes(), ...routes });
  const onSignOut = vi.fn();
  const user = userEvent.setup();
  render(<Dashboard api={adminApi(() => Promise.resolve("dev-token"))} me={ME} onSignOut={onSignOut} />);
  return { fake, onSignOut, user };
}

const status = (): HTMLElement => screen.getByTestId(testids.status);

const refusal = (message: string): Route => ({ status: 400, body: { error: { code: "invalid_request", message } } });

/** Waits for the reads a section makes on opening. */
async function loaded(): Promise<void> {
  await screen.findByTestId(testids.ratingValue("constructed"));
  await screen.findAllByTestId(testids.leaderboardRow);
  await screen.findAllByTestId(testids.newsRow);
  await screen.findAllByTestId(testids.auditRow);
  await screen.findAllByTestId(testids.questRow);
}

describe("Account", () => {
  it("shows who the server says the account is, and Sign out signs out", async () => {
    const { onSignOut, user } = open();
    await loaded();
    expect(screen.getByTestId(testids.accountUsername)).toHaveTextContent("devname");
    expect(screen.getByTestId(testids.accountProfileId)).toHaveTextContent("p-dev");
    expect(screen.getByTestId(testids.accountStrict)).toHaveTextContent(/^Yes/u);
    expect(screen.getByTestId(testids.accountSecondFactor)).toHaveTextContent("Yes");
    await user.click(screen.getByTestId(testids.signOut));
    expect(onSignOut).toHaveBeenCalledTimes(1);
  });

  it("every request carries the account's token", async () => {
    const { fake } = open();
    await loaded();
    expect(fake.calls.length).toBeGreaterThan(0);
    for (const call of fake.calls) expect(call.headers["authorization"]).toBe("Bearer dev-token");
  });
});

describe("Holdings and Dev tools", () => {
  it("shows the account's Jlore and held quests", async () => {
    open();
    await loaded();
    expect(screen.getByTestId(testids.walletJlore)).toHaveTextContent("1234");
    const rows = screen.getAllByTestId(testids.questRow);
    expect(rows.map((row) => row.dataset["questId"])).toEqual(["q-1", "q-2"]);
    expect(rows[0]).toHaveTextContent("Win 3 games: 1 of 3, pays 50 Jlore and 100 XP");
  });

  it("offers every shipped set's track", async () => {
    open();
    await loaded();
    const options = within(screen.getByTestId(testids.set)).getAllByRole("option");
    expect(options.map((option) => option.getAttribute("value"))).toEqual([...SHIPPED_SETS]);
  });

  it("Give XP sends the chosen set and the typed amount, and the status line shows the answer", async () => {
    const { fake, user } = open({ "POST /api/dev/xp": { body: { track: { setId: "Classic", xp: 250, level: 2, rewardsClaimed: 1 } } } });
    await loaded();
    await user.selectOptions(screen.getByTestId(testids.set), "Classic");
    await user.type(screen.getByTestId(testids.xpInput), "250");
    await user.click(screen.getByTestId(testids.xpGive));
    await waitFor(() => {
      expect(status()).toHaveAttribute("data-state", "done");
    });
    expect(fake.sent("POST", "/api/dev/xp")[0]?.body).toEqual({ setId: "Classic", xp: 250 });
    expect(status()).toHaveTextContent('Done: {"track":{"setId":"Classic","xp":250,"level":2,"rewardsClaimed":1}}');
  });

  it("Give Jlore sends the typed amount", async () => {
    const { fake, user } = open({ "POST /api/dev/jlore": { body: { jlore: 1734 } } });
    await loaded();
    await user.type(screen.getByTestId(testids.jloreInput), "500");
    await user.click(screen.getByTestId(testids.jloreGive));
    await waitFor(() => {
      expect(status()).toHaveTextContent('Done: {"jlore":1734}');
    });
    expect(fake.sent("POST", "/api/dev/jlore")[0]?.body).toEqual({ jlore: 500 });
  });

  it("every other tool sends its own route", async () => {
    const tools: [string, string][] = [
      [testids.fill, "/api/dev/fill"],
      [testids.unlockCosmetics, "/api/dev/cosmetics"],
      ...BARS.map((bar): [string, string] => [testids.bar(bar), `/api/dev/bar/${bar}`]),
      [testids.questGive, "/api/dev/quests"],
      ...CLEAR_PARTS.map((part): [string, string] => [testids.clear(part), `/api/dev/clear/${part}`]),
    ];
    const { fake, user } = open(Object.fromEntries(tools.map(([, path]) => [`POST ${path}`, { body: { written: 3 } }])));
    await loaded();
    for (const [testid, path] of tools) {
      await user.click(screen.getByTestId(testid));
      await waitFor(() => {
        expect(fake.sent("POST", path), path).toHaveLength(1);
        expect(status()).toHaveAttribute("data-state", "done");
      });
      expect(fake.sent("POST", path)[0]?.body).toEqual({});
    }
    expect(screen.getByTestId(testids.fill)).toHaveTextContent("Add all cosmetics and cards");
  });

  it("Complete quest sends the held quest chosen", async () => {
    const { fake, user } = open({ "POST /api/dev/quests/q-2/complete": { body: { completed: "q-2", jlore: 20, xp: 40 } } });
    await loaded();
    const select = screen.getByTestId(testids.questSelect);
    expect(within(select).getAllByRole("option").map((option) => option.textContent)).toEqual(
      QUESTS.quests.map((quest) => quest.title),
    );
    await user.selectOptions(select, "q-2");
    await user.click(screen.getByTestId(testids.questComplete));
    await waitFor(() => {
      expect(fake.sent("POST", "/api/dev/quests/q-2/complete")).toHaveLength(1);
    });
  });

  it("shows a refusal word for word", async () => {
    const { user } = open({ "POST /api/dev/jlore": refusal("Jlore is a positive whole number.") });
    await loaded();
    await user.click(screen.getByTestId(testids.jloreGive));
    await waitFor(() => {
      expect(status()).toHaveAttribute("data-state", "failed");
    });
    expect(status()).toHaveTextContent("Jlore is a positive whole number.");
  });

  it("holds every action while a request is out", async () => {
    const { user } = open({ "POST /api/dev/fill": "never" });
    await loaded();
    await user.click(screen.getByTestId(testids.fill));
    expect(status()).toHaveTextContent("Working…");
    for (const testid of [testids.xpGive, testids.jloreGive, testids.questGive, testids.ratingSet("random"), testids.newsPost]) {
      expect(screen.getByTestId(testid), testid).toBeDisabled();
    }
  });

  it("an answered request reads every section again", async () => {
    const { fake, user } = open({ "POST /api/dev/fill": { body: { cards: 300, cosmetics: 80 } } });
    await loaded();
    const reads = (): number => fake.calls.filter((call) => call.method === "GET").length;
    const before = reads();
    await user.click(screen.getByTestId(testids.fill));
    await waitFor(() => {
      expect(reads()).toBe(before + Object.keys(dashboardRoutes()).length - 2);
    });
  });
});

describe("Rating", () => {
  it("shows each ladder's rating, deviation, volatility, hand-set flag and earned rating", async () => {
    open();
    await loaded();
    expect(screen.getByTestId(testids.ratingValue("constructed"))).toHaveTextContent("1800");
    expect(screen.getByTestId(testids.ratingDeviation("constructed"))).toHaveTextContent("50.12");
    expect(screen.getByTestId(testids.ratingVolatility("constructed"))).toHaveTextContent("0.06");
    expect(screen.getByTestId(testids.ratingHandSet("constructed"))).toHaveTextContent("Yes");
    expect(screen.getByTestId(testids.ratingEarned("constructed"))).toHaveTextContent("1523.46 (deviation 80, volatility 0.06)");
    expect(screen.getByTestId(testids.ratingValue("random"))).toHaveTextContent("1500");
    expect(screen.getByTestId(testids.ratingHandSet("random"))).toHaveTextContent("No");
    expect(screen.getByTestId(testids.ratingEarned("random"))).toHaveTextContent("None kept");
    expect(screen.getByTestId(testids.rating)).toHaveTextContent(
      "On a strict server a hand-set rating is off the public ladder until reset.",
    );
  });

  it("offers Reset to earned only on a hand-set ladder", async () => {
    open();
    await loaded();
    expect(screen.getByTestId(testids.ratingReset("constructed"))).toBeInTheDocument();
    expect(screen.queryByTestId(testids.ratingReset("random"))).toBeNull();
  });

  it("Set sends the ladder and the typed rating; Reset sends the ladder", async () => {
    const { fake, user } = open({
      "PUT /api/admin/rating": { body: { ladders: [] } },
      "POST /api/admin/rating/reset": { body: { ladders: [] } },
    });
    await loaded();
    await user.type(screen.getByTestId(testids.ratingInput("random")), "1650.5");
    await user.click(screen.getByTestId(testids.ratingSet("random")));
    await waitFor(() => {
      expect(fake.sent("PUT", "/api/admin/rating")[0]?.body).toEqual({ ladder: "random", rating: 1650.5 });
    });
    await waitFor(() => {
      expect(status()).toHaveAttribute("data-state", "done");
    });
    await user.click(screen.getByTestId(testids.ratingReset("constructed")));
    await waitFor(() => {
      expect(fake.sent("POST", "/api/admin/rating/reset")[0]?.body).toEqual({ ladder: "constructed" });
    });
  });

  it("shows the server's range when it refuses a rating", async () => {
    const { user } = open({ "PUT /api/admin/rating": refusal("A rating is a number from 100 to 3000.") });
    await loaded();
    await user.type(screen.getByTestId(testids.ratingInput("constructed")), "99999");
    await user.click(screen.getByTestId(testids.ratingSet("constructed")));
    await waitFor(() => {
      expect(status()).toHaveTextContent("A rating is a number from 100 to 3000.");
    });
  });
});

describe("Leaderboard", () => {
  it("shows every row, people and bots marked, a bot's strength and persona beside it, a hand-set rating marked", async () => {
    open();
    await loaded();
    const rows = screen.getAllByTestId(testids.leaderboardRow);
    expect(rows.map((row) => [row.dataset["profileId"], row.dataset["kind"]])).toEqual(
      CONSTRUCTED.rows.map((row) => [row.profileId, row.kind]),
    );
    const [you, bot, retired, placing] = rows;
    expect(you).toHaveTextContent("devname (you)");
    expect(you).toHaveAttribute("data-you", "true");
    expect(you).toHaveAttribute("data-hand-set", "true");
    expect(you).toHaveTextContent("Hand-set");
    expect(you).toHaveTextContent("Golden · division 2 · 3/5 pips");
    expect(bot).toHaveTextContent("Bot");
    expect(bot).toHaveTextContent("strength 0.75, aggro");
    expect(bot).toHaveTextContent("25 / 14 / 1");
    expect(bot).toHaveTextContent("1650.5");
    expect(bot).toHaveAttribute("data-hand-set", "false");
    expect(retired).toHaveTextContent("strength 0.2, control, retired");
    expect(retired).toHaveTextContent("Jlorious · #4");
    expect(placing).toHaveTextContent("Person");
    expect(placing).toHaveTextContent("Normal · placements 3/10");
    expect(screen.getByTestId(testids.leaderboardSeason)).toHaveTextContent("Season 2026-11");
    // Read-only: a row holds no control.
    for (const row of rows) {
      expect(within(row).queryAllByRole("button")).toEqual([]);
      expect(row.querySelector("input, select, textarea")).toBeNull();
    }
  });

  it("switches ladder", async () => {
    const { fake, user } = open();
    await loaded();
    expect(screen.getByTestId(testids.ladder("constructed"))).toHaveAttribute("aria-pressed", "true");
    await user.click(screen.getByTestId(testids.ladder("random")));
    await waitFor(() => {
      expect(screen.getAllByTestId(testids.leaderboardRow).map((row) => row.dataset["profileId"])).toEqual(["p-rand"]);
    });
    expect(fake.sent("GET", "/api/admin/leaderboard?ladder=random")).toHaveLength(1);
    expect(screen.getByTestId(testids.ladder("random"))).toHaveAttribute("aria-pressed", "true");
  });
});

describe("News posts", () => {
  it("lists every post as plain text, newest first, a taken-down one without Take down", async () => {
    open();
    await loaded();
    const rows = screen.getAllByTestId(testids.newsRow);
    expect(rows.map((row) => row.dataset["postId"])).toEqual(NEWS.posts.map((post) => post.id));
    const [live, gone] = rows;
    expect(live).toHaveAttribute("data-taken-down", "false");
    expect(within(live as HTMLElement).getByTestId(testids.newsTakeDown)).toBeInTheDocument();
    expect(live).toHaveTextContent(`Posted ${when(AT)} by devname`);
    expect(gone).toHaveAttribute("data-taken-down", "true");
    expect(within(gone as HTMLElement).queryByTestId(testids.newsTakeDown)).toBeNull();
    expect(gone).toHaveTextContent("<b>Old news</b>");
    expect(gone).toHaveTextContent("an account that is gone");
    expect((gone as HTMLElement).querySelector("b, img")).toBeNull();
  });

  it("says a post pays no reward", async () => {
    open();
    await loaded();
    expect(screen.getByTestId(testids.news)).toHaveTextContent(NO_REWARD_NOTE);
  });

  it("Post sends the title and summary as typed, and clears the form once posted", async () => {
    const { fake, user } = open({ "POST /api/admin/news": { status: 201, body: { post: NEWS.posts[0] } } });
    await loaded();
    await user.type(screen.getByTestId(testids.newsTitle), "Server move");
    await user.type(screen.getByTestId(testids.newsSummary), "We move tonight. Expect a short break!");
    await user.click(screen.getByTestId(testids.newsPost));
    await waitFor(() => {
      expect(status()).toHaveTextContent("Posted.");
    });
    expect(fake.sent("POST", "/api/admin/news")[0]?.body).toEqual({
      title: "Server move",
      summary: "We move tonight. Expect a short break!",
    });
    expect(screen.getByTestId(testids.newsTitle)).toHaveValue("");
    expect(screen.getByTestId(testids.newsSummary)).toHaveValue("");
  });

  it("keeps the form and shows the server's words when it refuses a post", async () => {
    const { user } = open({ "POST /api/admin/news": refusal("A summary is one or two sentences.") });
    await loaded();
    await user.type(screen.getByTestId(testids.newsTitle), "T");
    await user.type(screen.getByTestId(testids.newsSummary), "no stop");
    await user.click(screen.getByTestId(testids.newsPost));
    await waitFor(() => {
      expect(status()).toHaveTextContent("A summary is one or two sentences.");
    });
    expect(screen.getByTestId(testids.newsSummary)).toHaveValue("no stop");
  });

  it("Take down sends the post's id", async () => {
    const { fake, user } = open({ "DELETE /api/admin/news/n-2": { body: { post: { ...NEWS.posts[0], takenDownAt: AT } } } });
    await loaded();
    await user.click(screen.getByTestId(testids.newsTakeDown));
    await waitFor(() => {
      expect(fake.sent("DELETE", "/api/admin/news/n-2")).toHaveLength(1);
    });
    await waitFor(() => {
      expect(status()).toHaveTextContent("Taken down.");
    });
  });
});

describe("Audit", () => {
  it("lists every action with its request, before and after as JSON text", async () => {
    open();
    await loaded();
    const rows = screen.getAllByTestId(testids.auditRow);
    expect(rows.map((row) => row.dataset["actionId"])).toEqual(["7", "6"]);
    const [latest, older] = rows;
    const action = AUDIT.actions[0];
    expect(latest).toHaveTextContent("devname");
    expect(latest).toHaveTextContent("PUT /api/admin/rating");
    expect(latest).toHaveTextContent(json(action?.request));
    expect(latest).toHaveTextContent(json(action?.before));
    expect(latest).toHaveTextContent(json(action?.after));
    expect(latest).toHaveTextContent(when(AT));
    // An account that is gone is named by its profile id.
    expect(older).toHaveTextContent("p-gone");
    expect(older).toHaveTextContent("null");
  });

  it("Refresh reads it again", async () => {
    const { fake, user } = open();
    await loaded();
    await user.click(screen.getByTestId(testids.auditRefresh));
    await waitFor(() => {
      expect(fake.sent("GET", "/api/admin/audit")).toHaveLength(2);
    });
  });
});
