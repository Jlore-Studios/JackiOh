// A friend's lobby invite as a toast (R3730005): Open goes to the lobby, Dismiss sends the toast away. R3729002: the
// tab keeps the invite for the bell's inbox, so neither answer forgets it (`inbox/invites.ts`).

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import type { ReactElement } from "react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { dropInvite, forgetInvites, useInvites } from "../inbox/invites.ts";
import { at } from "../test/stubServer.ts";
import { LobbyInviteToast } from "./LobbyInviteToast.tsx";
import { lobbyTestid } from "./testids.ts";

const listeners: ((invite: { code: string; format: "duel" | "coop"; from: { profileId: string; username: string } }) => void)[] = [];

vi.mock("../together/socialPoll.ts", () => ({
  onLobbyInvitePush: (listener: (typeof listeners)[number]) => {
    listeners.push(listener);
    return () => {
      listeners.splice(listeners.indexOf(listener), 1);
    };
  },
}));

/** What the tab holds for the inbox, as the bell reads it. */
function Held(): ReactElement {
  return (
    <ul>
      {useInvites().map((held) => (
        <li key={held.code} data-testid="held-invite" data-code={held.code} data-toast-hidden={String(held.toastHidden)} />
      ))}
    </ul>
  );
}

function invite(code: string, format: "duel" | "coop"): void {
  act(() => {
    for (const listener of [...listeners]) listener({ code, format, from: { profileId: "p-ana", username: "Ana#1" } });
  });
}

beforeEach(() => {
  at("/collection");
});

afterEach(() => {
  cleanup();
  forgetInvites();
});

describe("the lobby invite toast", () => {
  it("R3730005 an invite shows who invites to which kind of lobby; Open goes there and Dismiss sends the toast away", () => {
    render(<LobbyInviteToast />);
    expect(screen.queryByTestId(lobbyTestid.inviteToast)).toBeNull();
    invite("K7Q2XM", "duel");
    invite("COOPCD", "coop");
    const toasts = screen.getAllByTestId(lobbyTestid.inviteToast);
    expect(toasts.map((toast) => toast.textContent)).toEqual([
      expect.stringContaining("Ana#1 invites you to a 1v1 lobby"),
      expect.stringContaining("Ana#1 invites you to a co-op lobby"),
    ]);
    fireEvent.click(screen.getAllByTestId(lobbyTestid.inviteDismiss)[1] as HTMLElement);
    expect(screen.getAllByTestId(lobbyTestid.inviteToast)).toHaveLength(1);
    fireEvent.click(screen.getByTestId(lobbyTestid.inviteOpen));
    expect(window.location.pathname).toBe("/lobby/K7Q2XM");
    expect(screen.queryByTestId(lobbyTestid.inviteToast)).toBeNull();
  });

  it("R3729002 Dismiss hides the toast and the store keeps the invite for the inbox; Open hides it and goes", () => {
    render(
      <>
        <LobbyInviteToast />
        <Held />
      </>,
    );
    invite("K7Q2XM", "duel");
    invite("COOPCD", "coop");
    expect(screen.getAllByTestId("held-invite").map((held) => held.getAttribute("data-toast-hidden"))).toEqual(["false", "false"]);

    fireEvent.click(screen.getAllByTestId(lobbyTestid.inviteDismiss)[0] as HTMLElement);
    expect(screen.getAllByTestId(lobbyTestid.inviteToast)).toHaveLength(1);
    expect(screen.getAllByTestId("held-invite").map((held) => [held.getAttribute("data-code"), held.getAttribute("data-toast-hidden")])).toEqual([
      ["K7Q2XM", "true"],
      ["COOPCD", "false"],
    ]);

    fireEvent.click(screen.getByTestId(lobbyTestid.inviteOpen));
    expect(window.location.pathname).toBe("/lobby/COOPCD");
    expect(screen.queryByTestId(lobbyTestid.inviteToast)).toBeNull();
    expect(screen.getAllByTestId("held-invite")).toHaveLength(2);

    // The inbox's own answers drop an invite for good; a second frame for the same lobby shows its toast again.
    act(() => {
      dropInvite("K7Q2XM");
    });
    expect(screen.getAllByTestId("held-invite").map((held) => held.getAttribute("data-code"))).toEqual(["COOPCD"]);
    invite("COOPCD", "coop");
    expect(screen.getAllByTestId(lobbyTestid.inviteToast)).toHaveLength(1);
  });
});
