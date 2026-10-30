package wake

// wake.py's behaviour, against answers taken from the Python (bot/tests/test_wake.py's cases and
// wake_time minute by minute around both changes of the clocks).

import (
	"strings"
	"testing"
	"time"
	_ "time/tzdata"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
)

// needPorted skips a test while a package it leans on is still the generated stub (each package
// is ported on its own; the tests run in full once they are merged).
func needPorted(t *testing.T, calls ...func()) {
	t.Helper()
	for _, call := range calls {
		if herr.Try(call) != nil {
			t.Skip("a package this test needs is not ported in this tree")
		}
	}
}

func usesClock() { clock.ISO(time.Unix(0, 0)); clock.Zone(Zone) }

func utc(text string) time.Time {
	t, err := time.Parse(time.RFC3339, text)
	if err != nil {
		panic(err)
	}
	return t
}

func TestWakeTimeAroundTheChangesOfTheClocks(t *testing.T) {
	needPorted(t, usesClock)
	// From each minute on, until the next row: wake_time's answer (UTC) and how the Python
	// prints it (`{at:%H:%M}`), or none.
	segments := []struct{ from, at, hhmm string }{
		{"2026-10-31T00:00:00Z", "2026-10-30T23:00:00Z", "18:00"},
		{"2026-10-31T01:00:00Z", "", ""},
		{"2026-10-31T02:00:00Z", "2026-10-31T03:00:00Z", "22:00"},
		{"2026-10-31T05:00:00Z", "", ""},
		{"2026-10-31T06:00:00Z", "2026-10-31T07:00:00Z", "02:00"},
		{"2026-10-31T09:00:00Z", "", ""},
		{"2026-10-31T10:00:00Z", "2026-10-31T11:00:00Z", "06:00"},
		{"2026-10-31T13:00:00Z", "", ""},
		{"2026-10-31T14:00:00Z", "2026-10-31T15:00:00Z", "10:00"},
		{"2026-10-31T17:00:00Z", "", ""},
		{"2026-10-31T18:00:00Z", "2026-10-31T19:00:00Z", "14:00"},
		{"2026-10-31T21:00:00Z", "", ""},
		{"2026-10-31T22:00:00Z", "2026-10-31T23:00:00Z", "18:00"},
		{"2026-11-01T01:00:00Z", "", ""},
		{"2026-11-01T02:00:00Z", "2026-11-01T03:00:00Z", "22:00"},
		{"2026-11-01T05:00:00Z", "", ""},
		{"2026-11-01T06:00:00Z", "2026-11-01T08:00:00Z", "02:00"},
		{"2026-11-01T10:00:00Z", "", ""},
		{"2026-11-01T11:00:00Z", "2026-11-01T12:00:00Z", "06:00"},
		{"2026-11-01T14:00:00Z", "", ""},
		{"2026-11-01T15:00:00Z", "2026-11-01T16:00:00Z", "10:00"},
		{"2026-11-01T18:00:00Z", "", ""},
		{"2026-11-01T19:00:00Z", "2026-11-01T20:00:00Z", "14:00"},
		{"2026-11-01T22:00:00Z", "", ""},
		{"2026-11-01T23:00:00Z", "2026-11-02T00:00:00Z", "18:00"},
		{"2026-11-02T02:00:00Z", "", ""},
		{"2026-11-02T03:00:00Z", "2026-11-02T04:00:00Z", "22:00"},
		{"2026-11-02T06:00:00Z", "", ""},
		{"2026-11-02T07:00:00Z", "2026-11-02T08:00:00Z", "02:00"},
		{"2026-11-02T10:00:00Z", "", ""},
		{"2026-11-02T11:00:00Z", "2026-11-02T12:00:00Z", "06:00"},
		{"2026-11-02T12:00:00Z", "end", ""},
		{"2027-03-13T00:00:00Z", "2027-03-13T00:00:00Z", "18:00"},
		{"2027-03-13T02:00:00Z", "", ""},
		{"2027-03-13T03:00:00Z", "2027-03-13T04:00:00Z", "22:00"},
		{"2027-03-13T06:00:00Z", "", ""},
		{"2027-03-13T07:00:00Z", "2027-03-13T08:00:00Z", "02:00"},
		{"2027-03-13T10:00:00Z", "", ""},
		{"2027-03-13T11:00:00Z", "2027-03-13T12:00:00Z", "06:00"},
		{"2027-03-13T14:00:00Z", "", ""},
		{"2027-03-13T15:00:00Z", "2027-03-13T16:00:00Z", "10:00"},
		{"2027-03-13T18:00:00Z", "", ""},
		{"2027-03-13T19:00:00Z", "2027-03-13T20:00:00Z", "14:00"},
		{"2027-03-13T22:00:00Z", "", ""},
		{"2027-03-13T23:00:00Z", "2027-03-14T00:00:00Z", "18:00"},
		{"2027-03-14T02:00:00Z", "", ""},
		{"2027-03-14T03:00:00Z", "2027-03-14T04:00:00Z", "22:00"},
		{"2027-03-14T06:00:00Z", "", ""},
		// 02:00 does not exist that night: Python reads it with the offset before the change
		// (08:00Z) and prints the wall time it was given; Go prints that instant as 03:00 CDT.
		{"2027-03-14T07:00:00Z", "2027-03-14T08:00:00Z", "03:00"},
		{"2027-03-14T09:00:00Z", "", ""},
		{"2027-03-14T10:00:00Z", "2027-03-14T11:00:00Z", "06:00"},
		{"2027-03-14T13:00:00Z", "", ""},
		{"2027-03-14T14:00:00Z", "2027-03-14T15:00:00Z", "10:00"},
		{"2027-03-14T17:00:00Z", "", ""},
		{"2027-03-14T18:00:00Z", "2027-03-14T19:00:00Z", "14:00"},
		{"2027-03-14T21:00:00Z", "", ""},
		{"2027-03-14T22:00:00Z", "2027-03-14T23:00:00Z", "18:00"},
		{"2027-03-15T01:00:00Z", "", ""},
		{"2027-03-15T02:00:00Z", "2027-03-15T03:00:00Z", "22:00"},
		{"2027-03-15T05:00:00Z", "", ""},
		{"2027-03-15T06:00:00Z", "2027-03-15T07:00:00Z", "02:00"},
		{"2027-03-15T09:00:00Z", "", ""},
		{"2027-03-15T10:00:00Z", "2027-03-15T11:00:00Z", "06:00"},
		{"2027-03-15T12:00:00Z", "end", ""},
	}
	for i := 0; i+1 < len(segments); i++ {
		s := segments[i]
		if s.at == "end" {
			continue
		}
		for now := utc(s.from); now.Before(utc(segments[i+1].from)); now = now.Add(time.Minute) {
			got := WakeTime(now)
			if s.at == "" {
				if !got.IsZero() {
					t.Fatalf("wake_time(%s): got %s, want None", now, got)
				}
				continue
			}
			if !got.Equal(utc(s.at)) || got.Format("15:04") != s.hhmm {
				t.Fatalf("wake_time(%s): got %s (%s), want %s (%s)", now, got.UTC(), got.Format("15:04"), s.at, s.hhmm)
			}
		}
	}
}

func TestAlreadyWoken(t *testing.T) {
	needPorted(t, usesClock)
	cdtSix := utc("2026-10-22T11:00:00Z")
	run := func(conclusion string, at time.Time) *pyjson.Object {
		return pyjson.New("conclusion", conclusion, "updated_at", clock.ISO(at))
	}
	green := run("success", cdtSix.Add(2*time.Minute))
	early := run("success", cdtSix.Add(-2*time.Hour))
	failed := run("failure", cdtSix.Add(2*time.Minute))
	later := cdtSix.Add(30 * time.Minute)
	if !AlreadyWoken([]*pyjson.Object{early, green}, later) || AlreadyWoken([]*pyjson.Object{early, failed}, later) {
		t.Error("a green run since the wake time woke the sessions; a red one did not")
	}
	if AlreadyWoken([]*pyjson.Object{green}, cdtSix.Add(Every-47*time.Minute)) ||
		AlreadyWoken([]*pyjson.Object{green}, cdtSix.Add(3*time.Hour)) {
		t.Error("the last wake does not count for the next")
	}
}

// pool is four Claude subscriptions and one on the machine.
func pool(t *testing.T) *providers.Pool {
	raw, err := pyjson.LoadsString(`{"max_parallel": 2, "tiers": {"strong": ["opus"], "medium": ["gpt"]},
	  "providers": {
	    "claude-1": {"cli": "claude", "family": "claude", "model": "opus", "tier": "strong",
	      "secret": "CLAUDE_CODE_OAUTH_TOKEN", "schedule": {"mode": "always"}, "limits": {"mode": "none"}},
	    "claude-2": {"cli": "claude", "family": "claude", "model": "opus", "tier": "strong",
	      "secret": "CLAUDE_CODE_OAUTH_TOKEN_2", "schedule": {"mode": "always"}, "limits": {"mode": "none"}},
	    "gpt": {"cli": "codex", "family": "gpt", "model": "gpt", "tier": "medium", "login": "machine",
	      "runs_on": "night-vm-gpt", "schedule": {"mode": "always"}, "limits": {"mode": "none"}},
	    "claude-3": {"cli": "claude", "family": "claude", "model": "opus", "tier": "strong",
	      "secret": "CLAUDE_CODE_OAUTH_TOKEN_3", "schedule": {"mode": "always"}, "limits": {"mode": "none"}},
	    "claude-4": {"cli": "claude", "family": "claude", "model": "opus", "tier": "strong", "enabled": false,
	      "secret": "CLAUDE_CODE_OAUTH_TOKEN_4", "schedule": {"mode": "always"}, "limits": {"mode": "none"}}}}`)
	if err != nil {
		t.Fatal(err)
	}
	return providers.Parse(raw)
}

func TestAccounts(t *testing.T) {
	needPorted(t, usesClock)
	cdtSix := utc("2026-10-22T11:00:00Z")
	iso := func(d time.Duration) string { return clock.ISO(cdtSix.Add(d)) }
	reading := func(used float64, resets time.Duration) *pyjson.Object {
		return pyjson.New("utilization", used, "resets_at", iso(resets))
	}
	cfg := &config.Config{Pool: pool(t), Secrets: providers.SecretsOf(map[string]string{
		"HARNESS_SECRETS_SET": "CLAUDE_CODE_OAUTH_TOKEN CLAUDE_CODE_OAUTH_TOKEN_3"})}
	found, notes := Accounts(cfg, pyjson.New("providers", pyjson.New()), cdtSix, ".harness")
	if got := pyjson.Dumps(found); got != `[{"id": "claude-1", "secret": "CLAUDE_CODE_OAUTH_TOKEN", "home": ".harness"}, `+
		`{"id": "claude-3", "secret": "CLAUDE_CODE_OAUTH_TOKEN_3", "home": ".harness"}]` {
		t.Errorf("the matrix: %s", got)
	}
	if want := "`claude-2`: no `CLAUDE_CODE_OAUTH_TOKEN_2` secret|`claude-4`: turned off"; strings.Join(notes, "|") != want {
		t.Errorf("notes: %q", notes)
	}
	cfg.Secrets = providers.SecretsOf(map[string]string{})
	state := pyjson.New("providers", pyjson.New(
		"claude-1", pyjson.New("usage", pyjson.New("five_hour", reading(0.3, 2*time.Hour))),
		"claude-2", pyjson.New("suspended", pyjson.New("by", "jgoetzmann")),
		"claude-3", pyjson.New("refused_until", iso(3*time.Hour), "usage", pyjson.New(
			"five_hour", reading(0.9, -5*time.Minute), "seven_day", reading(1, 48*time.Hour)))))
	found, notes = Accounts(cfg, state, cdtSix, ".squishy")
	if len(found) != 0 {
		t.Errorf("found %s", pyjson.Dumps(found))
	}
	want := []string{
		"`claude-1`: its session is running until 2026-10-22T13:00:00Z",
		"`claude-2`: suspended",
		"`claude-3`: its usage limit refuses calls until 2026-10-24T11:00:00Z",
		"`claude-4`: turned off",
	}
	if strings.Join(notes, "\n") != strings.Join(want, "\n") {
		t.Errorf("notes:\n%s", strings.Join(notes, "\n"))
	}
	// A refusal and a full window that have run out hold nothing.
	state = pyjson.New("providers", pyjson.New("claude-1", pyjson.New("refused_until", iso(-5*time.Minute),
		"usage", pyjson.New("seven_day", reading(1.0, -5*time.Minute)))))
	if found, _ = Accounts(cfg, state, cdtSix, ".harness"); len(found) != 3 {
		t.Errorf("found %s", pyjson.Dumps(found))
	}
}

func TestWake(t *testing.T) {
	needPorted(t, usesClock, func() { (&config.Config{}).ClaudeBin() },
		func() { rateLimited("") })
	cfg := &config.Config{Pool: pool(t)}
	saved := Ping
	t.Cleanup(func() { Ping = saved })
	answer := func(result, usage string) {
		Ping = func(claudeBin string, model string, run runner.RunFunc, token string, effort string) (*pyjson.Object, *pyjson.Object) {
			if model != "claude-haiku-5-5" || effort != "low" || token != "" || run != nil {
				t.Errorf("ping(%q, %q, %v, %q, %q)", claudeBin, model, run != nil, token, effort)
			}
			var r, u *pyjson.Object
			if result != "" {
				v, _ := pyjson.LoadsString(result)
				r = v.(*pyjson.Object)
			}
			if usage != "" {
				v, _ := pyjson.LoadsString(usage)
				u = v.(*pyjson.Object)
			}
			return r, u
		}
	}
	for _, c := range []struct {
		result, usage string
		ok            bool
		line          string
	}{
		{"", `{"five_hour": {"utilization": 0.01, "resets_at": "2026-10-01T05:00:00Z"}}`, true,
			"`claude-1`: the 5-hour session is running, resetting at 2026-10-01T05:00:00Z"},
		{"", `{"status": "allowed"}`, true, "`claude-1`: the 5-hour session is running, resetting at an unknown time"},
		{`{"is_error": true, "result": "You've hit your limit"}`, `{"status": "rejected", "rejected_resets_at": "2026-10-01T05:00:00Z"}`,
			true, "`claude-1`: its usage limit refused the ping, so it is left until 2026-10-01T05:00:00Z; the next wake time tries it again"},
		{`{"is_error": true, "result": "Claude AI usage limit reached"}`, "", true,
			"`claude-1`: its usage limit refused the ping, so it is left until its reset; the next wake time tries it again"},
		{`{"is_error": true, "result": "connection reset"}`, "", false,
			"`claude-1`: the CLI gave no usage reading, so the session may not have started"},
		{`{"is_error": false, "result": "usage limit"}`, "{}", false,
			"`claude-1`: the CLI gave no usage reading, so the session may not have started"},
	} {
		answer(c.result, c.usage)
		if ok, line := Wake(cfg, "claude-1"); ok != c.ok || line != c.line {
			t.Errorf("wake with %s %s: got %v %q", c.result, c.usage, ok, line)
		}
	}
	if ok, line := Wake(cfg, "gpt"); ok || line != "`gpt` is not a Claude subscription here" {
		t.Errorf("wake gpt: %v %q", ok, line)
	}
}
