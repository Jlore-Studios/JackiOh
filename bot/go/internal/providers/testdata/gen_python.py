"""Records bot/harness/providers.py's answers for bot/go/internal/providers/testdata/python.json.

From bot/, with HARNESS_HOME unset:

    PYTHONPATH=. python3 go/internal/providers/testdata/gen_python.py \
        go/internal/providers/testdata/python.json 12 16

The two numbers are how many randomized usage scenarios and state writes to record (seeded, so
the same numbers give the same file); larger ones make a wider local check. Every case names an
op, its inputs and the Python's answer, and python_test.go replays it.
"""

import copy
import json
import random
import sys
from datetime import datetime, timedelta, timezone

from harness import clock, providers
from harness.errors import ConfigError

ZONE = "America/Chicago"
BASE = json.loads(open("../.harness/providers.json", encoding="utf-8").read())
rng = random.Random(467)


def iso(t):
    return clock.iso(t) if t is not None else None


def mutate(raw, ops):
    raw = copy.deepcopy(raw)
    for op in ops:
        node = raw
        for key in op["path"][:-1]:
            node = node[key]
        if op.get("del"):
            del node[op["path"][-1]]
        else:
            node[op["path"][-1]] = op["set"]
    return raw


cases = []

# ------------------------------------------------------------------ parse errors
ERRORS = [
    [{"path": ["providers", "gpt", "surprise"], "set": 1}],
    [{"path": ["providers", "gpt", "cli"], "set": "copilot"}],
    [{"path": ["providers", "gpt", "secret"], "set": "BOT_GITHUB_TOKEN"}],
    [{"path": ["providers", "gpt", "secret"], "set": "CODEX_AUTH_JSON"}],
    [{"path": ["providers", "gpt", "login"], "set": "secret"}, {"path": ["providers", "gpt", "secret"], "set": "BOT"}],
    [{"path": ["providers", "gpt", "login"], "set": None}],
    [{"path": ["providers", "gpt", "limits"], "set": {"mode": "caps", "five_hour": 98}}],
    [{"path": ["providers", "gpt", "limits"], "set": {"mode": "caps"}}],
    [{"path": ["providers", "gpt", "limits"], "set": {"mode": "caps", "seven_day_minutes": 0}}],
    [{"path": ["providers", "gpt", "limits"], "set": {"mode": "weird"}}],
    [{"path": ["providers", "gpt", "limits"], "set": "caps"}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "start": "9"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "start": "9", "end": "10:00"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "start": "09:00"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "end": "09:00"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "start": "25:00", "end": "09:00"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "start": "ab:00", "end": "09:00"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "window", "start": "1:2:3", "end": "09:00"}}],
    [{"path": ["providers", "gpt", "schedule"], "set": {"mode": "sometimes"}}],
    [{"path": ["providers", "gpt", "roles"], "set": ["build", "dance"]}],
    [{"path": ["providers", "gpt", "roles"], "set": "plan"}],
    [{"path": ["priority"], "set": ["claude-1"]}],
    [{"path": ["priority"], "set": list(BASE["priority"]) + ["claude-1"]}],
    [{"path": ["max_parallel"], "set": 0}],
    [{"path": ["machine_parallel"], "set": -1}],
    [{"path": ["plan_lanes"], "set": -1}],
    [{"path": ["balance"], "set": "yes"}],
    [{"path": ["balance"], "set": None}],
    [{"path": ["providers", "gpt", "tier"], "set": "genius"}],
    [{"path": ["providers", "gpt", "tier"], "del": True}],
    [{"path": ["providers", "gpt", "tier"], "set": "strong"}],
    [{"path": ["providers", "gpt", "model"], "set": "gpt-9"}],
    [{"path": ["tiers", "medium"], "set": BASE["tiers"]["medium"] + [{"model": "opus"}]}],
    [{"path": ["tiers", "huge"], "set": []}],
    [{"path": ["tiers"], "del": True}],
    [{"path": ["tiers", "medium"], "set": BASE["tiers"]["medium"] + [{"model": "x", "why": 1}]}],
    [{"path": ["tiers", "medium"], "set": BASE["tiers"]["medium"] + [{"note": "x"}]}],
    [{"path": ["providers", "claude-2", "extra_models"], "set": [{"model": "sonnet", "tier": "weak"}]}],
    [{"path": ["providers", "claude-2", "extra_models"], "set": [{"model": "sonnet", "tier": "medium", "takes_over": "nobody"}]}],
    [{"path": ["providers", "claude-2", "extra_models"], "set": [{"model": "sonnet", "tier": "medium", "takes_over": "claude-2"}]}],
    [{"path": ["providers", "claude-2", "extra_models"], "set": [{"tier": "weak"}]}],
    [{"path": ["providers", "claude-2", "extra_models"], "set": [{"model": "sonnet", "tier": "medium", "x": 1}]}],
    [{"path": ["providers", "claude-2", "extra_models"], "set": {"model": "sonnet"}}],
    [{"path": ["providers", "gpt", "difficult"], "set": True}],
    [{"path": ["providers", "gpt", "runs_on"], "set": "ubuntu-latest"}],
    [{"path": ["providers", "gpt", "runs_on"], "set": "night vm"}],
    [{"path": ["providers", "gpt", "login"], "set": "keyring"}],
    [{"path": ["providers", "agy", "login"], "set": "secret"}, {"path": ["providers", "agy", "secret"], "set": "MUSE_AUTH"}, {"path": ["providers", "agy", "runs_on"], "set": "ubuntu-latest"}],
    [{"path": ["providers", "muse", "runs_on"], "set": "night-vm-gpt"}],
    [{"path": ["providers", "claude-2", "runs_on"], "set": "night-vm-gpt"}],
    [{"path": ["providers", "gpt", "lanes"], "set": 0}],
    [{"path": ["providers", "gpt", "lanes"], "set": "two"}],
    [{"path": ["providers", "gpt", "lanes"], "set": None}],
    [{"path": ["providers", "gpt", "env"], "set": ["A"]}],
    [{"path": ["providers", "gpt", "quiet_check"], "set": True}],
    [{"path": ["providers", "gpt", "last_hour"], "set": True}, {"path": ["providers", "gpt", "limits"], "set": {"mode": "none"}}],
    [{"path": ["providers", "claude-1", "off_hours"], "set": {"five_hours": 0.4}}],
    [{"path": ["providers", "claude-1", "off_hours"], "set": {"five_hour": 1.4}}],
    [{"path": ["providers", "claude-1", "off_hours"], "set": [0.4]}],
    [{"path": ["providers", "claude-1", "start_headroom"], "set": {"five_hour": 1.0}}],
    [{"path": ["providers", "claude-1", "start_headroom"], "set": {"weekly": 0.1}}],
    [{"path": ["providers", "claude-1", "start_headroom"], "set": 0.1}],
    [{"path": ["start_headroom"], "set": {"five_hour": -0.1}}],
    [{"path": ["providers", "muse", "off_from"], "set": "mid-October"}],
    [{"path": ["providers", "muse", "off_from"], "set": "2026-13-01"}],
    [{"path": ["providers", "muse", "usage_since"], "set": "last Tuesday"}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "night-vm-muse-fill"}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "ubuntu-x", "for": ["gpt"]}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "night-vm-gpt", "for": ["gpt"]}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "fill", "for": ["nobody"]}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "fill", "for": ["muse"]}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "fill", "for": "gpt"}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "fill", "for": ["gpt", ""]}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "fill", "for": ["gpt"], "x": 1}}],
    [{"path": ["providers", "muse", "fill_in"], "set": {"runs_on": "fill", "for": ["gpt", "gpt"]}}],
    [{"path": ["providers", "gpt", "fill_in"], "set": {"runs_on": "fill2", "for": ["agy"]}}],
    [{"path": ["providers", "claude-1", "fill_in"], "set": {"runs_on": "fill2", "for": ["agy"]}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Someday 17:00", "seven_day": 0.9}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 25:00", "seven_day": 0.9}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 17:61", "seven_day": 0.9}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 17:00"}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 17:00", "seven_day": 0.9, "until": "Mon"}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 17:00", "seven_day": 0.5}}],
    [{"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 17:00", "seven_day": 1.0, "five_hour": 0.3}}],
    [{"path": ["providers", "claude-4", "late_week"], "set": {"from": "Sun 17:00", "seven_day": 0.9}}],
    [{"path": ["providers", "claude-3", "late_week"], "set": {"from": "Sun 17:00", "seven_day": 0.9, "last_hour": True}}],
    [{"path": ["providers", "claude-5", "limits"], "set": {"mode": "caps", "seven_day": 0.6}},
     {"path": ["providers", "claude-5", "late_week"], "set": {"from": "Sun 17:00", "seven_day": 1.0, "last_hour": True}}],
    [{"path": ["providers", "claude-1", "secret"], "set": "CLAUDE_CODE_OAUTH_TOKEN_2"}],
    [{"path": ["providers"], "set": {}}],
    [{"path": ["providers", "gpt"], "set": "gpt"}],
    [{"path": ["providers", "gpt", "family"], "del": True}],
]
for ops in ERRORS:
    try:
        providers.parse(mutate(BASE, ops))
        error = None
    except ConfigError as exc:
        error = str(exc)
    cases.append({"op": "parse_error", "mutate": ops, "error": error})
try:
    providers.parse([1, 2])
except ConfigError as exc:
    cases.append({"op": "parse_error", "raw": [1, 2], "error": str(exc)})

# ------------------------------------------------------------------ pools to ask about
VARIANTS = {
    "base": [],
    "budgets": [{"path": ["providers", "muse", "limits"], "set": {"mode": "caps", "seven_day_minutes": 60, "five_hour_minutes": 30}},
                {"path": ["providers", "claude-4", "off_hours"], "del": True},
                {"path": ["providers", "gpt", "off_from"], "set": "2026-11-01"},
                {"path": ["providers", "agy", "enabled"], "set": False},
                {"path": ["providers", "muse", "usage_since"], "set": None},
                {"path": ["providers", "claude-2", "start_headroom"], "set": {"five_hour": 0.2}},
                {"path": ["providers", "claude-1", "env"], "set": {"B": 2, "A": "x"}},
                {"path": ["providers", "claude-1", "roles"], "set": ["build", "plan"]}],
}
POOLS = {name: providers.parse(mutate(BASE, ops)) for name, ops in VARIANTS.items()}


def summary(pool):
    out = {"priority": list(pool.priority), "max_parallel": pool.max_parallel,
           "machine_parallel": pool.machine_parallel, "plan_lanes": pool.plan_lanes,
           "balance": pool.balance,
           "tiers": {t: [e.model for e in es] for t, es in pool.tiers.items()},
           "providers": {}}
    for p in pool.ordered():
        out["providers"][p.id] = {
            "describe": p.describe(), "hours": p.hours(ZONE), "enabled": p.enabled, "cli": p.cli,
            "family": p.family, "model": p.model, "effort": p.effort, "secret": p.secret,
            "tier": p.tier, "login": p.login, "runs_on": p.runs_on, "lanes": p.lanes,
            "roles": list(p.roles), "env": dict(sorted(p.env.items())),
            "stops": dict(p.limits.stops), "budgets": dict(p.limits.budgets),
            "headroom": dict(p.limits.headroom), "off_hours": dict(p.off_hours),
            "off_from": str(p.off_from) if p.off_from else None,
            "usage_since": iso(p.usage_since), "fix_effort": p.fix_effort,
            "self_check": p.self_check, "build_last": p.build_last, "last_hour": p.last_hour,
            "takes_last_hours": p.takes_last_hours,
            "late_week": None if p.late_week is None else {
                "describe": p.late_week.describe(), "caps": p.late_week.caps,
                "weekday": p.late_week.weekday, "last_hour": p.late_week.last_hour},
            "fill_in": None if p.fill_in is None else {"runs_on": p.fill_in.runs_on, "of": list(p.fill_in.of)},
            "seats": [s.to_dict() | {"rank": s.rank, "takes_over": s.takes_over, "describe": s.describe()}
                      for s in pool.seats(p)],
            "own_seats": [s.model for s in pool.own_seats(p)],
            "best_seat": {f: (s.model if (s := pool.best_seat(p, f)) else None) for f in providers.TIERS},
            "filler_of": (f.id if (f := pool.filler_of(p.id)) else None),
            "filled_by": pool.filled_by(p), "on_machine": pool.on_machine(p.id),
            "schedule": [p.schedule.mode, p.schedule.start, p.schedule.end],
        }
    out["family_tier"] = {f: pool.family_tier(f) for f in ("claude", "gpt", "gemini", "muse", "none")}
    out["seat"] = [[i, m, (s.model if (s := pool.seat(i, m)) else None)]
                   for i, m in (("claude-1", None), ("claude-1", "sonnet"), ("claude-1", "x"), ("nobody", None))]
    return out


for name, pool in POOLS.items():
    cases.append({"op": "summary", "pool": name, "want": summary(pool)})

# ------------------------------------------------------------------ usage scenarios
ANCHORS = [datetime(2026, 9, 29, 17, 0, tzinfo=timezone.utc), datetime(2026, 9, 30, 3, 0, tzinfo=timezone.utc),
           datetime(2026, 10, 9, 6, 0, tzinfo=timezone.utc), datetime(2026, 11, 1, 23, 0, tzinfo=timezone.utc),
           datetime(2026, 11, 1, 6, 30, tzinfo=timezone.utc), datetime(2026, 10, 25, 21, 59, tzinfo=timezone.utc),
           datetime(2027, 3, 14, 8, 30, tzinfo=timezone.utc), datetime(2026, 10, 11, 22, 0, tzinfo=timezone.utc)]
UTILS = [0.1, 0.3, 0.39, 0.4, 0.41, 0.45, 0.49, 0.5, 0.55, 0.62, 0.65, 0.69, 0.7, 0.72, 0.78, 0.8, 0.81,
         0.85, 0.89, 0.9, 0.92, 0.95, 0.96, 0.99, 1.0, 1, 1.05, True]


def reading(at):
    r = {}
    if rng.random() < 0.8:
        r["utilization"] = rng.choice(UTILS)
    if rng.random() < 0.85:
        r["resets_at"] = iso(at + timedelta(minutes=rng.randint(-120, 600)))
    return r


def entry_for(at):
    entry = {}
    if rng.random() < 0.85:
        usage = {}
        if rng.random() < 0.85:
            usage["five_hour"] = reading(at)
        if rng.random() < 0.7:
            usage["seven_day"] = reading(at) if rng.random() < 0.6 else {
                "utilization": rng.choice(UTILS), "resets_at": iso(at + timedelta(hours=rng.randint(-5, 170)))}
        if rng.random() < 0.6:
            usage["observed_at"] = iso(at - timedelta(minutes=rng.randint(0, 600)))
        if rng.random() < 0.05:
            usage = "corrupt"
        entry["usage"] = usage
    if rng.random() < 0.15:
        entry["refused_until"] = iso(at + timedelta(minutes=rng.randint(-60, 300)))
    if rng.random() < 0.15:
        entry["infra"] = {"at": iso(at - timedelta(minutes=rng.randint(0, 600))),
                          "reason": "401 OAuth token refused " + "é" * rng.randint(0, 130),
                          "streak": rng.choice([1, 2, 3, 4, "x", None, 2.7])}
    if rng.random() < 0.3:
        entry["spent"] = [{"at": iso(at - timedelta(hours=rng.randint(0, 200))), "minutes": rng.choice([5, 12.5, 30, "7"])}
                          for _ in range(rng.randint(1, 6))] + [{"at": None}, "junk"]
    return entry


SECRET_SETS = [" ".join(providers.SECRETS), "CLAUDE_CODE_OAUTH_TOKEN", "", None]

for n in range(int(sys.argv[2])):
    pool_name = rng.choice(list(POOLS))
    pool = POOLS[pool_name]
    anchor = rng.choice(ANCHORS)
    at = anchor + timedelta(minutes=rng.randint(-900, 900))
    state = {"providers": {}}
    for p in pool.ordered():
        if rng.random() < 0.6:
            state["providers"][p.id] = entry_for(at)
    if rng.random() < 0.2:
        state["usage"] = {"five_hour": {"utilization": 1.0, "resets_at": iso(at + timedelta(hours=1))}}
        state["rate_limited_until"] = iso(at + timedelta(minutes=30))
    secret_set = rng.choice(SECRET_SETS)
    env = {} if secret_set is None else {"HARNESS_SECRETS_SET": secret_set}
    if secret_set is None and rng.random() < 0.5:
        env = {"CLAUDE_CODE_OAUTH_TOKEN_3": "x"}
    secrets = providers.Secrets.of(env)
    case = {"op": "scenario", "pool": pool_name, "at": iso(at), "state": state, "env": env,
            "when_free": providers.when_free(pool, state, at, ZONE, secrets), "providers": {}}
    for p in pool.ordered():
        entry = providers.peek_record(state, p.id)
        last_hour = at + timedelta(minutes=30) if rng.random() < 0.2 else None
        usage = providers.current_usage(p, entry)
        frees = providers.frees_at(p, entry, at, ZONE)
        case["providers"][p.id] = {
            "last_hour": iso(last_hour),
            "availability": [providers.availability(p, state, at, ZONE, secrets, forced=f, starting=s)
                             for f in (False, True) for s in (False, True)],
            "start_reason": providers.start_reason(p, state, at, ZONE, secrets),
            "refusal": [providers.refusal(p, entry, at, ZONE, starting=s, last_hour=last_hour)
                        for s in (False, True)],
            "refusal_no_zone": providers.refusal(p, entry, at, None),
            "room": providers.room(p, entry, at, ZONE),
            "frees_at": iso(frees),
            "usage_frees_at": iso(providers.usage_frees_at(
                p, state, at, ZONE, providers.availability(p, state, at, ZONE, secrets, starting=True))),
            "caps_at": p.caps_at(at, ZONE, usage, last_hour=last_hour),
            "caps_at_none": p.caps_at(at, ZONE),
            "held_caps": p.held_caps(at, ZONE, usage),
            "last_hour_of": iso(providers.last_hour_of(p, usage, at, ZONE)),
            "late_week_holds": providers.late_week_holds(p, usage, at, ZONE),
            "late_week_since": iso(p.late_week.since(at, ZONE)) if p.late_week else None,
            "switched_off": providers.switched_off_by_date(p, at, ZONE),
            "near_full_reset": iso(providers.near_full_reset(entry.get("usage"), at)),
            "minutes_spent": providers.minutes_spent(entry, at - timedelta(days=7)),
            "infra_backoff": providers.infra_backoff(entry.get("infra") if isinstance(entry.get("infra"), dict) else {}).total_seconds(),
        }
    cases.append(case)

# ------------------------------------------------------------------ writes to the state
AT = datetime(2026, 10, 3, 23, 47, tzinfo=timezone.utc)
for n in range(int(sys.argv[3])):
    state = {"providers": {}}
    if rng.random() < 0.5:
        state["providers"]["claude-1"] = entry_for(AT)
    if rng.random() < 0.5:
        state["usage"] = {"five_hour": {"utilization": 0.5}}
    if rng.random() < 0.5:
        state["rate_limited_until"] = iso(AT + timedelta(minutes=10))
    if rng.random() < 0.4:
        state["providers"]["gpt"] = {"usage": {"seven_day": {"utilization": rng.choice([0.4, 0.86, 0.89, True]),
                                                             "resets_at": "2026-10-09T23:47:06Z"},
                                               "five_hour": {"utilization": 0.9, "resets_at": "2026-10-04T01:00:00Z"}},
                                     "spent": [{"at": "x", "minutes": 1}] * rng.randint(0, 3),
                                     "infra": {"issue": rng.choice([None, 12, "13"]), "streak": rng.choice([1, 3])}}
    pid = rng.choice(["claude-1", "gpt", "muse"])
    usage = rng.choice([None, {}, {"five_hour": {"utilization": 0.2}}, {"status": "allowed", "seven_day": {"utilization": 0.3}}])
    reset_at = rng.choice([None, "", "+PT30M", "+PT60M", "+PT1H30M", "+pt5m", "+PT", "+PTxM", "2026-10-05T00:00:00Z", "nonsense", "+PT١٢M"])
    minutes = rng.choice([0, 0.0, 12.34, 25, 0.05, 7.25])
    tokens = rng.choice([None, {}, {"input": 0, "output": 0}, {"input": 1200, "output": 30, "cache_read": 5, "cache_write": 7},
                         {"input": 3.9, "output": "4"}])
    cost = rng.choice([None, 0.0, 0.123456, 2])
    record_before = copy.deepcopy(state)
    providers.note_usage(state, pid, usage, reset_at, AT, minutes, tokens, cost)
    case = {"op": "write", "state": record_before, "provider": pid, "usage": usage, "reset_at": reset_at,
            "minutes": minutes, "tokens": tokens, "cost": cost, "after_usage": json.dumps(state)}
    before_infra = copy.deepcopy(state)
    escalate = rng.random() < 0.6
    reason = "refused " + "x" * rng.randint(0, 480) + "ü" * rng.randint(0, 40)
    streak = providers.note_infra(state, pid, reason, AT, escalate=escalate)
    case.update(escalate=escalate, reason=reason, streak=streak, after_infra=json.dumps(state))
    state2 = copy.deepcopy(state)
    case["cleared"] = providers.clear_infra(state2, pid)
    case["after_clear"] = json.dumps(state2)
    peeked = copy.deepcopy(record_before)
    case["peek"] = json.dumps(providers.peek_record(peeked, "claude-1"))
    case["peek_unchanged"] = json.dumps(peeked)
    recorded = copy.deepcopy(record_before)
    providers.record(recorded, "claude-1")
    case["record"] = json.dumps(recorded)
    cases.append(case)

# ------------------------------------------------------------------ secrets
NOW = datetime(2026, 9, 30, 3, 0, tzinfo=timezone.utc)
for env in ({"HARNESS_SECRETS_SET": "B A  C"}, {"HARNESS_SECRETS_SET": ""}, {"CLAUDE_CODE_OAUTH_TOKEN": "x", "MUSE_AUTH": ""}, {}):
    secrets = providers.Secrets.of(env)
    for state in ({}, {"secrets": {"set": ["A", "B", "C"], "at": iso(NOW - timedelta(minutes=10))}},
                  {"secrets": {"set": ["A", "B", "C"], "at": iso(NOW - timedelta(minutes=40))}},
                  {"secrets": {"set": ["A"], "at": iso(NOW - timedelta(minutes=10))}},
                  {"secrets": {"set": "A", "at": iso(NOW)}}, {"secrets": "x"}):
        for since in (None, NOW - timedelta(minutes=20)):
            newer = providers.newer_secrets(secrets, state, since)
            cases.append({"op": "secrets", "env": env, "state": state, "since": iso(since), "now": iso(NOW),
                          "record": providers.secrets_record(secrets, state, NOW),
                          "newer": [sorted(newer.present), newer.known],
                          "has": [secrets.has(n) for n in ("A", "CLAUDE_CODE_OAUTH_TOKEN", "MUSE_AUTH")]})

for text in ("+PT90M", "+PT2H", "+PT1H30M", " +pt10s ", "PT5M", "+PT", "bogus", "", "+PT0M", "+PT١٢M", "+PT1H2M3S"):
    cases.append({"op": "duration", "text": text, "seconds": providers._duration(text).total_seconds()})

json.dump({"providers_path": str(providers.PROVIDERS_PATH), "base": BASE, "variants": VARIANTS,
           "zone": ZONE, "write_at": iso(AT), "cases": cases}, open(sys.argv[1], "w"),
          separators=(",", ":"))
print(len(cases), "cases")
