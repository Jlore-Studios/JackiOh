package providers

// Replays testdata/python.json: what bot/harness/providers.py answered for a snapshot of
// `.harness/providers.json` (parse errors, each pool's seats and texts, availability and its
// texts over randomized usage readings, the writes to the state file, the secrets records).
// The file is written by running the Python; regenerate it whenever providers.py changes.

import (
	"fmt"
	"os"
	"testing"
	"time"
	_ "time/tzdata"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// needClock skips a test while clock is still the generated stub (each package is ported on its
// own; the tests run in full once they are merged).
func needClock(t *testing.T) {
	t.Helper()
	if herr.Try(func() { clock.ISO(time.Unix(0, 0)) }) != nil {
		t.Skip("clock is not ported in this tree")
	}
}

func loadPython(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

func mutate(base any, ops []any) any {
	raw := pyjson.DeepCopy(base)
	for _, item := range ops {
		op := item.(*pyjson.Object)
		path := op.List("path")
		node := raw.(*pyjson.Object)
		for _, key := range path[:len(path)-1] {
			node = node.Obj(key.(string))
		}
		last := path[len(path)-1].(string)
		if op.Truthy("del") {
			node.Delete(last)
		} else {
			node.Set(last, op.Get("set"))
		}
	}
	return raw
}

func isoOrNil(t time.Time) any {
	if t.IsZero() {
		return nil
	}
	return clock.ISO(t)
}

func strOrNil(s string) any {
	if s == "" {
		return nil
	}
	return s
}

func at(v any) time.Time {
	if v == nil {
		return time.Time{}
	}
	return clock.ParseISO(v.(string))
}

func env(o *pyjson.Object) map[string]string {
	out := map[string]string{}
	o.Items(func(k string, v any) bool {
		out[k] = v.(string)
		return true
	})
	return out
}

func check(t *testing.T, where string, got, want any) {
	t.Helper()
	if !pyjson.Equal(normal(got), want) {
		t.Errorf("%s:\n got %s\nwant %s", where, pyjson.Dumps(normal(got)), pyjson.Dumps(want))
	}
}

// normal turns Go values into JSON values for pyjson.Equal.
func normal(v any) any {
	switch x := v.(type) {
	case []string:
		return pyjson.ListOf(x)
	case []any:
		out := make([]any, len(x))
		for i, e := range x {
			out[i] = normal(e)
		}
		return out
	case *pyjson.Object:
		if x == nil {
			return nil
		}
		out := pyjson.New()
		x.Items(func(k string, e any) bool {
			out.Set(k, normal(e))
			return true
		})
		return out
	case *bool:
		if x == nil {
			return nil
		}
		return *x
	}
	return v
}

func summary(pool *Pool, zone string) *pyjson.Object {
	tiers := pyjson.New()
	pool.Tiers.Items(func(name string, v any) bool {
		var models []any
		for _, e := range v.([]*TierEntry) {
			models = append(models, e.Model)
		}
		tiers.Set(name, append([]any{}, models...))
		return true
	})
	out := pyjson.New("priority", pool.Priority, "max_parallel", pool.MaxParallel,
		"machine_parallel", pool.MachineParallel, "plan_lanes", pool.PlanLanes,
		"balance", pool.Balance, "tiers", tiers)
	each := pyjson.New()
	for _, p := range pool.Ordered() {
		names := map[string]bool{}
		for k := range p.Env {
			names[k] = true
		}
		envs := pyjson.New()
		for _, k := range sortedNames(names) {
			envs.Set(k, p.Env[k])
		}
		var late any
		if p.LateWeek != nil {
			late = pyjson.New("describe", p.LateWeek.Describe(), "caps", p.LateWeek.Caps(),
				"weekday", p.LateWeek.Weekday, "last_hour", p.LateWeek.LastHour)
		}
		var fill any
		if p.FillIn != nil {
			fill = pyjson.New("runs_on", p.FillIn.RunsOn, "of", p.FillIn.Of)
		}
		var seats, own []any
		for _, s := range pool.Seats(p) {
			seats = append(seats, s.ToDict().Set("rank", s.Rank).Set("takes_over", s.TakesOver).
				Set("describe", s.Describe()))
		}
		for _, s := range pool.OwnSeats(p) {
			own = append(own, s.Model)
		}
		best := pyjson.New()
		for _, floor := range Tiers {
			if s := pool.BestSeat(p, floor); s != nil {
				best.Set(floor, s.Model)
			} else {
				best.Set(floor, nil)
			}
		}
		var filler any
		if f := pool.FillerOf(p.ID); f != nil {
			filler = f.ID
		}
		var offFrom any
		if !p.OffFrom.IsZero() {
			offFrom = p.OffFrom.Format("2006-01-02")
		}
		each.Set(p.ID, pyjson.New(
			"describe", p.Describe(), "hours", p.Hours(zone), "enabled", p.Enabled, "cli", p.CLI,
			"family", p.Family, "model", p.Model, "effort", p.Effort, "secret", p.Secret,
			"tier", p.Tier, "login", p.Login, "runs_on", p.RunsOn, "lanes", p.Lanes,
			"roles", p.Roles, "env", envs,
			"stops", p.Limits.Stops(), "budgets", p.Limits.Budgets(),
			"headroom", p.Limits.Headroom, "off_hours", p.OffHours,
			"off_from", offFrom, "usage_since", isoOrNil(p.UsageSince), "fix_effort", p.FixEffort,
			"self_check", p.SelfCheck, "build_last", p.BuildLast, "last_hour", p.LastHour,
			"takes_last_hours", p.TakesLastHours(), "late_week", late, "fill_in", fill,
			"seats", append([]any{}, seats...), "own_seats", append([]any{}, own...), "best_seat", best,
			"filler_of", filler, "filled_by", pool.FilledBy(p), "on_machine", pool.OnMachine(p.ID),
			"schedule", []any{p.Schedule.Mode, p.Schedule.Start, p.Schedule.End}))
	}
	out.Set("providers", each)
	families := pyjson.New()
	for _, f := range []string{"claude", "gpt", "gemini", "muse", "none"} {
		families.Set(f, pool.FamilyTier(f))
	}
	out.Set("family_tier", families)
	var seat []any
	for _, q := range [][2]string{{"claude-1", ""}, {"claude-1", "sonnet"}, {"claude-1", "x"}, {"nobody", ""}} {
		var model any
		if s := pool.Seat(q[0], q[1]); s != nil {
			model = s.Model
		}
		seat = append(seat, []any{q[0], strOrNil(q[1]), model})
	}
	out.Set("seat", seat)
	return out
}

func TestPython(t *testing.T) {
	needClock(t)
	data := loadPython(t)
	saved := ProvidersPath
	ProvidersPath = data.Str("providers_path")
	t.Cleanup(func() { ProvidersPath = saved })
	base := data.Get("base")
	zone := data.Str("zone")
	pools := map[string]*Pool{}
	data.Obj("variants").Items(func(name string, ops any) bool {
		pools[name] = Parse(mutate(base, ops.([]any)))
		return true
	})
	for i, item := range data.List("cases") {
		c := item.(*pyjson.Object)
		where := fmt.Sprintf("case %d (%s)", i, c.Str("op"))
		switch c.Str("op") {
		case "parse_error":
			raw := c.Get("raw")
			if c.Has("mutate") {
				raw = mutate(base, c.List("mutate"))
			}
			var got any
			if e, ok := herr.Catch[*herr.ConfigError](func() { Parse(raw) }); ok {
				got = e.Msg
			}
			check(t, where, got, c.Get("error"))
		case "summary":
			check(t, where+" "+c.Str("pool"), summary(pools[c.Str("pool")], zone), c.Get("want"))
		case "scenario":
			replayScenario(t, where, c, pools[c.Str("pool")], zone)
		case "write":
			replayWrite(t, where, c, at(data.Get("write_at")))
		case "secrets":
			secrets := SecretsOf(env(c.Obj("env")))
			state := c.Obj("state")
			check(t, where+" record", SecretsRecord(secrets, state, at(c.Get("now"))), c.Get("record"))
			newer := NewerSecrets(secrets, state, at(c.Get("since")))
			check(t, where+" newer", []any{sortedNames(newer.Present), newer.Known}, c.Get("newer"))
			check(t, where+" has", []any{secrets.Has("A"), secrets.Has("CLAUDE_CODE_OAUTH_TOKEN"),
				secrets.Has("MUSE_AUTH")}, c.Get("has"))
		case "duration":
			check(t, where+" "+py.Repr(c.Str("text")), Duration(c.Str("text")).Seconds(), c.Get("seconds"))
		default:
			t.Fatalf("%s: unknown op", where)
		}
	}
}

func replayScenario(t *testing.T, where string, c *pyjson.Object, pool *Pool, zone string) {
	now := at(c.Get("at"))
	state := c.Obj("state")
	secrets := SecretsOf(env(c.Obj("env")))
	where += " at " + c.Str("at")
	check(t, where+" when_free", WhenFree(pool, state, now, zone, secrets), c.Get("when_free"))
	for _, p := range pool.Ordered() {
		want := c.Obj("providers").Obj(p.ID)
		w := where + " " + p.ID
		entry := PeekRecord(state, p.ID)
		lastHour := at(want.Get("last_hour"))
		usage := CurrentUsage(p, entry)
		var availability []any
		for _, forced := range []bool{false, true} {
			for _, starting := range []bool{false, true} {
				availability = append(availability, strOrNil(Availability(p, state, now, zone, secrets, forced, starting)))
			}
		}
		check(t, w+" availability", availability, want.Get("availability"))
		check(t, w+" start_reason", strOrNil(StartReason(p, state, now, zone, secrets)), want.Get("start_reason"))
		check(t, w+" refusal", []any{strOrNil(Refusal(p, entry, now, zone, false, lastHour)),
			strOrNil(Refusal(p, entry, now, zone, true, lastHour))}, want.Get("refusal"))
		check(t, w+" refusal_no_zone", strOrNil(Refusal(p, entry, now, "", false, time.Time{})), want.Get("refusal_no_zone"))
		if got, wantRoom := Room(p, entry, now, zone), want.Float("room"); got != wantRoom {
			t.Errorf("%s room: got %v want %v", w, got, wantRoom)
		}
		check(t, w+" frees_at", isoOrNil(FreesAt(p, entry, now, zone)), want.Get("frees_at"))
		check(t, w+" usage_frees_at", isoOrNil(UsageFreesAt(p, state, now, zone,
			Availability(p, state, now, zone, secrets, false, true))), want.Get("usage_frees_at"))
		check(t, w+" caps_at", p.CapsAt(now, zone, usage, lastHour), want.Get("caps_at"))
		check(t, w+" caps_at_none", p.CapsAt(now, zone, nil, time.Time{}), want.Get("caps_at_none"))
		check(t, w+" held_caps", p.HeldCaps(now, zone, usage), want.Get("held_caps"))
		check(t, w+" last_hour_of", isoOrNil(LastHourOf(p, usage, now, zone)), want.Get("last_hour_of"))
		check(t, w+" late_week_holds", LateWeekHolds(p, usage, now, zone), want.Get("late_week_holds"))
		var since any
		if p.LateWeek != nil {
			since = isoOrNil(p.LateWeek.Since(now, zone))
		}
		check(t, w+" late_week_since", since, want.Get("late_week_since"))
		check(t, w+" switched_off", SwitchedOffByDate(p, now, zone), want.Get("switched_off"))
		check(t, w+" near_full_reset", isoOrNil(NearFullReset(entry.Get("usage"), now)), want.Get("near_full_reset"))
		check(t, w+" minutes_spent", MinutesSpent(entry, now.Add(-7*24*time.Hour)), want.Get("minutes_spent"))
		infra, _ := mapping(entry.Get("infra"))
		check(t, w+" infra_backoff", InfraBackoff(infra).Seconds(), want.Get("infra_backoff"))
	}
}

func replayWrite(t *testing.T, where string, c *pyjson.Object, now time.Time) {
	state := pyjson.DeepCopy(c.Obj("state")).(*pyjson.Object)
	pid := c.Str("provider")
	var cost *float64
	if f, ok := pyjson.AsFloat(c.Get("cost")); ok {
		cost = &f
	}
	NoteUsage(state, pid, c.Obj("usage"), c.Str("reset_at"), now, c.Float("minutes"), c.Obj("tokens"), cost)
	check(t, where+" after_usage", pyjson.Dumps(state), c.Get("after_usage"))
	streak := NoteInfra(state, pid, c.Str("reason"), now, c.Bool("escalate"))
	check(t, where+" streak", streak, c.Get("streak"))
	check(t, where+" after_infra", pyjson.Dumps(state), c.Get("after_infra"))
	cleared := ClearInfra(state, pid)
	var got any
	if cleared != nil {
		got = *cleared
	}
	check(t, where+" cleared", got, c.Get("cleared"))
	check(t, where+" after_clear", pyjson.Dumps(state), c.Get("after_clear"))
	peeked := pyjson.DeepCopy(c.Obj("state")).(*pyjson.Object)
	check(t, where+" peek", pyjson.Dumps(PeekRecord(peeked, "claude-1")), c.Get("peek"))
	check(t, where+" peek_unchanged", pyjson.Dumps(peeked), c.Get("peek_unchanged"))
	recorded := pyjson.DeepCopy(c.Obj("state")).(*pyjson.Object)
	Record(recorded, "claude-1")
	check(t, where+" record", pyjson.Dumps(recorded), c.Get("record"))
}
