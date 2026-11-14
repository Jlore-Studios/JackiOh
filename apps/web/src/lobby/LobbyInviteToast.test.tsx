// A friend's lobby invite as a toast (R3730005): Open goes to the lobby, Dismiss drops it.

import { act, cleanup, fireEvent, render, screen } from "@testing-library/react";
import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";

import { at } from "../test/stubServer.ts";
import { LobbyInviteToast } from "./LobbyInviteToast.tsx";
import { lobbyTestid } from "./testids.ts";

const listeners: ((invite: { code: string; format: "duel" | "coop" | "2v2"; from: { profileId: string; username: string } }) => void)[] = [];

vi.mock("../together/socialPoll.ts", () => ({
  onLobbyInvitePush: (listener: (typeof listeners)[number]) => {
    listeners.push(listener);
    return () => {
      listeners.splice(listeners.indexOf(listener), 1);
    };
  },
}));

function invite(code: string, format: "duel" | "coop" | "2v2"): void {
  act(() => {
    for (const listener of [...listeners]) listener({ code, format, from: { profileId: "p-ana", username: "Ana#1" } });
  });
}

beforeEach(() => {
  at("/collection");
});

afterEach(() => {
  cleanup();
});

describe("the lobby invite toast", () => {
  it("R3730005 an invite shows who invites to which kind of lobby; Open goes there and Dismiss drops it", () => {
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

  it("#3731 an invite to a 2v2 lobby says so", () => {
    render(<LobbyInviteToast />);
    invite("TWOVTW", "2v2");
    expect(screen.getByTestId(lobbyTestid.inviteToast)).toHaveTextContent("Ana#1 invites you to a 2v2 lobby");
  });
});
