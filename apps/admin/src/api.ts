// The dashboard's one door onto the server (CLAUDE.md rule 7): every request is same-origin `/api/...`
// (the Worker hands it to the site's, worker/index.ts), carries the account's bearer token, and sends
// what the dev typed. The server decides; a refusal's `message` is shown to the dev word for word.

import type {
  AdminMe,
  AuditResponse,
  Bar,
  ClearPart,
  Ladder,
  LeaderboardResponse,
  NewsPost,
  NewsResponse,
  QuestView,
  QuestsResponse,
  RatingResponse,
  WalletResponse,
} from "./types.ts";
import { API_REQUEST_TIMEOUT_SECONDS, MS_PER_SECOND, type SetName } from "./wire.ts";

/** A refusal from the server, `{ "error": { "code", "message" } }`, carrying both verbatim. */
export class ApiError extends Error {
  readonly status: number;
  readonly code: string;

  constructor(status: number, code: string, message: string) {
    super(message);
    this.name = "ApiError";
    this.status = status;
    this.code = code;
  }
}

/** No answer, or none within the server's `API_REQUEST_TIMEOUT_SECONDS`. */
export const UNREACHABLE_MESSAGE = "Couldn't reach the server. Check your connection, then try again.";

export class ApiUnreachableError extends Error {
  constructor() {
    super(UNREACHABLE_MESSAGE);
    this.name = "ApiUnreachableError";
  }
}

/** What a status line shows for a failed request: the server's own message, or the unreachable one. */
export function messageOf(cause: unknown): string {
  return cause instanceof Error && cause.message.length > 0 ? cause.message : UNREACHABLE_MESSAGE;
}

function refusalOf(status: number, parsed: unknown): ApiError {
  const error = typeof parsed === "object" && parsed !== null ? (parsed as { error?: unknown }).error : undefined;
  if (typeof error === "object" && error !== null) {
    const { code, message } = error as { code?: unknown; message?: unknown };
    if (typeof code === "string" && typeof message === "string") return new ApiError(status, code, message);
  }
  return new ApiError(status, "internal", `The server answered ${String(status)}.`);
}

export type Method = "GET" | "POST" | "PUT" | "DELETE";

/** One request: the parsed body on 2xx, else `ApiError` (or `ApiUnreachableError`), so no caller reads a status. */
export async function apiRequest<T>(path: string, token: string, method: Method = "GET", body?: unknown): Promise<T> {
  const headers: Record<string, string> = { authorization: `Bearer ${token}` };
  if (body !== undefined) headers["content-type"] = "application/json";

  // Raced against our own timer as well as aborted, so a `fetch` that ignores its signal still gives up.
  const controller = new AbortController();
  let timer: ReturnType<typeof setTimeout> | undefined;
  const timedOut = new Promise<never>((_resolve, reject) => {
    timer = setTimeout(() => {
      controller.abort();
      reject(new ApiUnreachableError());
    }, API_REQUEST_TIMEOUT_SECONDS * MS_PER_SECOND);
  });

  let response: Response;
  let text: string;
  try {
    response = await Promise.race([
      fetch(path, {
        method,
        headers,
        ...(body === undefined ? {} : { body: JSON.stringify(body) }),
        signal: controller.signal,
      }),
      timedOut,
    ]);
    text = await Promise.race([response.text(), timedOut]);
  } catch {
    throw new ApiUnreachableError();
  } finally {
    clearTimeout(timer);
  }

  const parsed = parsedOrNull(text);
  if (!response.ok) throw refusalOf(response.status, parsed);
  return parsed as T;
}

/** A body as JSON; an empty or malformed one is `null` (a refusal's then names its status). */
function parsedOrNull(text: string): unknown {
  if (text.length === 0) return null;
  try {
    return JSON.parse(text) as unknown;
  } catch {
    return null;
  }
}

/** The account's current access token, renewed first when it is about to expire (session.ts). */
export type TokenSource = () => Promise<string>;

/** Every route the dashboard calls, with the bodies the server reads. */
export function adminApi(token: TokenSource) {
  const call = async <T>(path: string, method: Method = "GET", body?: unknown): Promise<T> =>
    apiRequest<T>(path, await token(), method, body);
  return {
    me: () => call<AdminMe>("/api/admin/me"),
    rating: () => call<RatingResponse>("/api/admin/rating"),
    setRating: (ladder: Ladder, rating: number) => call<RatingResponse>("/api/admin/rating", "PUT", { ladder, rating }),
    resetRating: (ladder: Ladder) => call<RatingResponse>("/api/admin/rating/reset", "POST", { ladder }),
    leaderboard: (ladder: Ladder) =>
      call<LeaderboardResponse>(`/api/admin/leaderboard?ladder=${encodeURIComponent(ladder)}`),
    news: () => call<NewsResponse>("/api/admin/news"),
    postNews: (title: string, summary: string) => call<{ post: NewsPost }>("/api/admin/news", "POST", { title, summary }),
    takeDownNews: (id: string) => call<{ post: NewsPost }>(`/api/admin/news/${encodeURIComponent(id)}`, "DELETE"),
    audit: () => call<AuditResponse>("/api/admin/audit"),

    // The holdings and the Dev tools (SPEC §14.10, R1300000), with exactly apps/web/src/net/api.ts's bodies.
    wallet: () => call<WalletResponse>("/api/wallet"),
    quests: () => call<QuestsResponse>("/api/quests"),
    giveXp: (setId: SetName, xp: number) => call<{ track: unknown }>("/api/dev/xp", "POST", { setId, xp }),
    giveJlore: (jlore: number) => call<{ jlore: number }>("/api/dev/jlore", "POST", { jlore }),
    fill: () => call<{ cards: number; cosmetics: number }>("/api/dev/fill", "POST", {}),
    clear: (part: ClearPart) => call<{ written: number }>(`/api/dev/clear/${part}`, "POST", {}),
    fillBar: (bar: Bar) => call<{ written: number }>(`/api/dev/bar/${bar}`, "POST", {}),
    giveQuest: () => call<{ quest: QuestView }>("/api/dev/quests", "POST", {}),
    completeQuest: (questId: string) =>
      call<{ completed: string; jlore: number; xp: number }>(
        `/api/dev/quests/${encodeURIComponent(questId)}/complete`,
        "POST",
        {},
      ),
  };
}

export type AdminApi = ReturnType<typeof adminApi>;
