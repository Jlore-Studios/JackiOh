// R4039001: the one notification toast. Each row the social socket pushes replaces the toast before it; a lobby
// invite has its own toast and shows none here; a board shown holds the toast until it goes. The socket is
// `together/socialPoll.ts`'s `onNotificationPush`, mocked as `lobby/LobbyInviteToast.test.tsx` mocks its push.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { useMarkBoardShown } from "../game/boardShown.ts";
import type { FeedNotification } from "../net/notifications.ts";
import { at } from "../test/stubServer.ts";
import { NotificationToast } from "./NotificationToast.tsx";
import { inboxTestid } from "./testids.ts";

const listeners: ((notification: FeedNotification) => void)[] = [];

vi.mock("../together/socialPoll.ts", () => ({
  onNotificationPush: (listener: (notification: FeedNotification) => void) => {
    listeners.push(listener);
    return () => {
      listeners.splice(listeners.indexOf(listener), 1);
    };
  },
  reloadSocial: () => undefined,
}));

const ANN = { profileId: "f-ann", username: "Ann#2" };

function push(id: number, kind: FeedNotification["kind"], over: Partial<FeedNotification> = {}): void {
  const row: FeedNotification = { id, kind, actor: ANN, subject: `m-${String(id)}`, payload: {}, createdAt: Date.now(), read: false, ...over };
  act(() => {
    for (const listener of [...listeners]) listener(row);
  });
}

/** A board that comes and goes, as `Game.tsx` marks itself (`game/boardShown.ts`). */
function Board(): null {
  useMarkBoardShown();
  return null;
}

function Page({ board = false }: { board?: boolean }): ReactElement {
  return (
    <>
      {board ? <Board /> : null}
      <NotificationToast />
    </>
  );
}

beforeEach(() => {
  at("/collection");
});

afterEach(() => {
  cleanup();
  listeners.length = 0;
});

describe("R4039001 the notification toast", () => {
  it("R4039001 a pushed row shows one toast, the newest replacing it, none for a lobby invite; Dismiss sends it away and Open goes there", () => {
    render(<Page />);
    expect(screen.queryByTestId(inboxTestid.toast)).toBeNull();

    push(1, "friendRequest");
    expect(screen.getByTestId(inboxTestid.toast)).toHaveTextContent("Ann#2 sent you a friend request");
    push(2, "dailyMove");
    expect(screen.getAllByTestId(inboxTestid.toast)).toHaveLength(1);
    expect(screen.getByTestId(inboxTestid.toast)).toHaveAttribute("data-kind", "dailyMove");
    expect(screen.getByTestId(inboxTestid.toast)).toHaveTextContent("Your move against Ann#2");

    // A lobby invite has its own toast (`lobby/LobbyInviteToast.tsx`): the one shown stays.
    push(3, "lobbyInvite", { subject: "K7Q2XM", payload: { format: "duel" } });
    expect(screen.getByTestId(inboxTestid.toast)).toHaveAttribute("data-kind", "dailyMove");

    fireEvent.click(screen.getByTestId(inboxTestid.toastDismiss));
    expect(screen.queryByTestId(inboxTestid.toast)).toBeNull();

    push(4, "dailyMove", { subject: "m-44" });
    fireEvent.click(screen.getByTestId(inboxTestid.toastOpen));
    expect(window.location.pathname).toBe("/daily/m-44");
    expect(screen.queryByTestId(inboxTestid.toast)).toBeNull();
  });

  it("R4039001 while a board is shown the toast is held, and it shows once the board goes", () => {
    const { rerender } = render(<Page board />);
    push(5, "dailyChallenge", { payload: { mode: "random", perMoveDays: 3 } });
    expect(screen.queryByTestId(inboxTestid.toast)).toBeNull();
    push(6, "friendAccepted");
    expect(screen.queryByTestId(inboxTestid.toast)).toBeNull();
    rerender(<Page />);
    expect(screen.getByTestId(inboxTestid.toast)).toHaveTextContent("Ann#2 accepted your friend request");
  });
});
