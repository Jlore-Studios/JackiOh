/**
 * The player's game settings on the account (SPEC §9.1, R633, R634).
 *
 * The device keeps the switches, volumes and choices a player has set in the settings dialog in
 * `localStorage`. An active account keeps the same on the server too, so a player who signs in on
 * another device, or clears site data, finds them there, and the client merges the two copies
 * (R634). This file is the account's half: two routes, both `active` (a pending account has the
 * code screen and nothing else, §9.4), both keyed on the profile the verified token names and
 * never on anything in the body.
 *
 *   GET /api/settings  -> { settings: { groups } }
 *   PUT /api/settings  <- { groups: { <id>: { at, values } } }
 *                      -> { settings: { groups } } as it stands after the merge
 *
 * A group is one of the client's stores (gameplay, audio, effects, card display): `values` is a
 * flat object of its settings and `at` the time, on the writing device's clock, that it last
 * changed. A PUT merges and never replaces (R634): each group sent replaces the stored one only
 * when its `at` is strictly later, and a group the write does not name is left alone, so a stale
 * device can never undo a newer change and a change to one group never undoes another's. The same
 * body sent twice changes nothing, so a client may retry it freely. The merge is the store's
 * (`app.merge_player_settings`, migration 0018), under a lock on the profile, so two devices
 * writing at once cannot lose each other's group.
 *
 * The server does not know the settings: they are the client's, and a setting added there needs no
 * change here. So a body is checked for its shape only — at most `PLAYER_SETTINGS_GROUPS_MAX`
 * groups, each a slug of at most `PLAYER_SETTINGS_NAME_MAX_LENGTH` characters holding at most
 * `PLAYER_SETTINGS_KEYS_MAX` settings named by short words, each a boolean, a finite number or a
 * text of at most `PLAYER_SETTINGS_TEXT_MAX_LENGTH` characters — and an account holds at most
 * `PLAYER_SETTINGS_BYTES_MAX` bytes of them. None of it is a rule: nothing the engine does reads
 * a setting.
 */

import {
  PLAYER_SETTINGS_BYTES_MAX,
  PLAYER_SETTINGS_GROUPS_MAX,
  PLAYER_SETTINGS_KEYS_MAX,
  PLAYER_SETTINGS_NAME_MAX_LENGTH,
  PLAYER_SETTINGS_TEXT_MAX_LENGTH,
} from "../config";
import { callerProfile } from "./collection";
import { ApiError, badRequest, ok, route, type Route } from "./http";
import type { PlayerSettingValue, PlayerSettingsGroup, PlayerSettingsRow } from "./ports";

/** R633: a group id is a lower-case slug (`gameplay`, `audio`, `fx`, `cards`). */
const GROUP_ID_SHAPE = /^[a-z][a-z0-9]*(?:-[a-z0-9]+)*$/u;
/** R633: a setting's name is a camelCase or lower-case word (`dragToPlay`, `master`). */
const SETTING_NAME_SHAPE = /^[a-zA-Z][a-zA-Z0-9]*$/u;

/** What the client reads: `PlayerSettingsAccountCopy` in `apps/web/src/net/api.ts`. */
export type PlayerSettingsView = { groups: Record<string, PlayerSettingsGroup> };

function settingsView(row: PlayerSettingsRow | null): PlayerSettingsView {
  if (row === null) return { groups: {} };
  const groups: Record<string, PlayerSettingsGroup> = {};
  for (const [id, group] of Object.entries(row.groups)) groups[id] = { at: group.at, values: { ...group.values } };
  return { groups };
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === "object" && value !== null && !Array.isArray(value);
}

function readValue(group: string, name: string, value: unknown): PlayerSettingValue {
  if (typeof value === "boolean") return value;
  if (typeof value === "number" && Number.isFinite(value)) return value;
  if (typeof value === "string" && value.length <= PLAYER_SETTINGS_TEXT_MAX_LENGTH) return value;
  throw badRequest(
    `"${group}.${name}" must be a boolean, a number or a text of at most ${String(PLAYER_SETTINGS_TEXT_MAX_LENGTH)} characters`,
  );
}

/**
 * R633, R634: `groups` is an object of at most `PLAYER_SETTINGS_GROUPS_MAX` groups, each
 * `{ at, values }`: `at` a whole number of epoch milliseconds, the writing device's clock, and
 * `values` a flat object of at most `PLAYER_SETTINGS_KEYS_MAX` settings. An `at` after the server's
 * own clock is taken as now, as R320 does for a choice: a device whose clock runs ahead could
 * otherwise make its settings win over every later change for as long as its clock stays ahead.
 */
export function readGroups(body: Readonly<Record<string, unknown>>, now: number): Record<string, PlayerSettingsGroup> {
  const raw = body["groups"];
  if (!isRecord(raw)) throw badRequest('"groups" must be an object of setting groups');
  const ids = Object.keys(raw);
  if (ids.length > PLAYER_SETTINGS_GROUPS_MAX) {
    throw badRequest(`at most ${String(PLAYER_SETTINGS_GROUPS_MAX)} setting groups can be saved`);
  }
  const groups: Record<string, PlayerSettingsGroup> = {};
  for (const id of ids) {
    if (id.length > PLAYER_SETTINGS_NAME_MAX_LENGTH || !GROUP_ID_SHAPE.test(id)) {
      throw badRequest(
        `every group id must be a lower-case slug of at most ${String(PLAYER_SETTINGS_NAME_MAX_LENGTH)} characters`,
      );
    }
    const group = raw[id];
    if (!isRecord(group) || !isRecord(group["values"])) {
      throw badRequest(`"${id}" must be { at: epoch milliseconds, values: an object }`);
    }
    const at = group["at"];
    if (typeof at !== "number" || !Number.isSafeInteger(at) || at < 0) {
      throw badRequest(`"${id}.at" must be a whole number of epoch milliseconds`);
    }
    const names = Object.keys(group["values"]);
    if (names.length > PLAYER_SETTINGS_KEYS_MAX) {
      throw badRequest(`"${id}" can hold at most ${String(PLAYER_SETTINGS_KEYS_MAX)} settings`);
    }
    const values: Record<string, PlayerSettingValue> = {};
    for (const name of names) {
      if (name.length > PLAYER_SETTINGS_NAME_MAX_LENGTH || !SETTING_NAME_SHAPE.test(name)) {
        throw badRequest(
          `every setting name must be letters and digits, at most ${String(PLAYER_SETTINGS_NAME_MAX_LENGTH)} characters`,
        );
      }
      values[name] = readValue(id, name, group["values"][name]);
    }
    groups[id] = { at: Math.min(at, now), values };
  }
  return groups;
}

/** `GET` and `PUT /api/settings`. Both `active`, so a pending account gets 403 from each (§9.4). */
export function createSettingsRoutes(): Route[] {
  return [
    route("GET", "/api/settings", "active", async (req, deps) => {
      const profile = callerProfile(req);
      return ok({ settings: settingsView(await deps.store.playerSettings.get(profile.id)) });
    }),

    route("PUT", "/api/settings", "active", async (req, deps) => {
      const profile = callerProfile(req);
      const now = deps.timers.now();
      const groups = readGroups(req.body, now);

      const outcome = await deps.store.playerSettings.merge(
        { profileId: profile.id, groups, at: now },
        { maxGroups: PLAYER_SETTINGS_GROUPS_MAX, maxBytes: PLAYER_SETTINGS_BYTES_MAX },
      );
      if (outcome.kind === "limit") {
        throw new ApiError(
          "conflict",
          `This account already holds the most settings it can (${String(PLAYER_SETTINGS_GROUPS_MAX)} groups, ${String(PLAYER_SETTINGS_BYTES_MAX)} bytes).`,
          { groups: PLAYER_SETTINGS_GROUPS_MAX, bytes: PLAYER_SETTINGS_BYTES_MAX },
        );
      }
      deps.log.info("settings.merged", {
        profileId: profile.id,
        sent: Object.keys(groups).length,
        held: Object.keys(outcome.settings.groups).length,
      });
      return ok({ settings: settingsView(outcome.settings) });
    }),
  ];
}
