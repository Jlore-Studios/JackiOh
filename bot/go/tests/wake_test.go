// wake_test.go is bot/tests/test_wake.py: `harness.wake`: the start of the Claude sessions every
// four hours (claude-wake.yml).
package tests

import (
	"fmt"
	"path/filepath"
	"slices"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/wake"
)

// 06:00 in Chicago on a CDT day (11:00 UTC) and on a CST day (12:00 UTC).
var (
	wakeCDTSix = time.Date(2026, 10, 22, 11, 0, 0, 0, time.UTC)
	wakeCSTSix = time.Date(2026, 12, 22, 12, 0, 0, 0, time.UTC)
)

// SET
var wakeSet = func() string {
	names := []string{}
	for _, n := range []string{"", "_2", "_3", "_4", "_5", "_6", "_7", "_8"} {
		names = append(names, "CLAUDE_CODE_OAUTH_TOKEN"+n)
	}
	return strings.Join(names, " ")
}()

// The wake times of a CDT day and of a CST day, in UTC: 02:00, 06:00, …, 22:00 Chicago.
var wakeWakes = func() []time.Time {
	out := []time.Time{}
	for _, six := range []time.Time{wakeCDTSix, wakeCSTSix} {
		for n := -1; n < 5; n++ {
			out = append(out, six.Add(time.Duration(n)*wake.Every))
		}
	}
	return out
}()

// ------------------------------------------------------------------ WindowTests

func TestWake_TheWakeTimesAreSixAndEveryFourHoursAroundItInEitherSeason(t *testing.T) {
	basicsEqual(t, wake.Every, 4*time.Hour)
	for _, at := range wakeWakes {
		basicsEqual(t, wake.WakeTime(at), at)
		basicsEqual(t, wake.WakeTime(at.Add(-47*time.Minute)), at)
	}
	basicsEqual(t, wake.WakeTime(wakeCDTSix).In(clock.Zone(wake.Zone)).Hour(), 6)
}

func TestWake_EachRunIsForAtMostOneWakeTime(t *testing.T) {
	basicsEqual(t, (24*time.Hour)%wake.Every, time.Duration(0))
	basicsTrue(t, wake.Early+wake.Late <= wake.Every)
}

// `13 * * * *`: the firing in the hour before a wake time sleeps until it; the two after it still
// wake the sessions, should GitHub drop the first.
func TestWake_EveryWakeTimeHasAFiringThatSleepsUntilItAndTwoAfter(t *testing.T) {
	text := basicsReadFile(t, filepath.Join(config.RepoRoot, ".github", "workflows", wake.Workflow))
	basicsIn(t, `- cron: "13 * * * *"`, text)
	for _, at := range wakeWakes {
		for _, fired := range []time.Time{at.Add(-47 * time.Minute), at.Add(13 * time.Minute),
			at.Add(73 * time.Minute)} {
			basicsEqual(t, wake.WakeTime(fired), at, fired)
		}
	}
}

func TestWake_ALateRunWakesAtOnceAndAMuchLaterOneDoesNothing(t *testing.T) {
	basicsEqual(t, wake.WakeTime(wakeCDTSix.Add(40*time.Minute)), wakeCDTSix)
	basicsTrue(t, wake.WakeTime(wakeCDTSix.Add(wake.Late)).IsZero())
	basicsTrue(t, wake.WakeTime(wakeCDTSix.Add(-wake.Early-time.Minute)).IsZero())
}

// 2027-03-14: 02:00 Chicago does not exist, so it and 06:00 are three hours apart.
func TestWake_ASpringForwardNightKeepsItsWindowsApart(t *testing.T) {
	six := time.Date(2027, 3, 14, 11, 0, 0, 0, time.UTC)
	two := wake.WakeTime(six.Add(-3 * time.Hour))
	basicsFalse(t, two.IsZero())
	basicsEqual(t, wake.WakeTime(six.Add(-30*time.Minute)), six)
	basicsEqual(t, wake.WakeTime(two.Add(30*time.Minute)), two)
}

func TestWake_ARunThatFinishedGreenSinceTheWakeTimeHasWokenThem(t *testing.T) {
	green := pyjson.New("conclusion", "success", "updated_at", clock.ISO(wakeCDTSix.Add(2*time.Minute)))
	early := pyjson.New("conclusion", "success", "updated_at", clock.ISO(wakeCDTSix.Add(-2*time.Hour)))
	failed := pyjson.New("conclusion", "failure", "updated_at", clock.ISO(wakeCDTSix.Add(2*time.Minute)))
	later := wakeCDTSix.Add(30 * time.Minute)
	basicsTrue(t, wake.AlreadyWoken([]*pyjson.Object{early, green}, later))
	basicsFalse(t, wake.AlreadyWoken([]*pyjson.Object{early, failed}, later))
}

// A run sleeping until ten finds six's green run, which woke six's sessions, not ten's.
func TestWake_TheLastWakeDoesNotCountForTheNext(t *testing.T) {
	green := pyjson.New("conclusion", "success", "updated_at", clock.ISO(wakeCDTSix.Add(2*time.Minute)))
	basicsFalse(t, wake.AlreadyWoken([]*pyjson.Object{green}, wakeCDTSix.Add(wake.Every-47*time.Minute)))
	basicsFalse(t, wake.AlreadyWoken([]*pyjson.Object{green}, wakeCDTSix.Add(3*time.Hour)))
}

// ------------------------------------------------------------------ AccountTests

// wakeCfg is AccountTests.cfg (secrets=SET by default).
func wakeCfg(secrets string) *config.Config {
	return testsupport.MakeConfig(testsupport.Config{Env: map[string]string{"HARNESS_SECRETS_SET": secrets}})
}

// wakeField is {a[name] for a in found}, or [a[name] for a in found] in order.
func wakeField(found []*pyjson.Object, name string) []string {
	out := []string{}
	for _, a := range found {
		out = append(out, a.Str(name))
	}
	return out
}

func TestWake_EveryClaudeSubscriptionWithItsSecretIsWoken(t *testing.T) {
	found, notes := wake.Accounts(wakeCfg(wakeSet), pyjson.New("providers", pyjson.New()), wakeCDTSix, ".harness")
	ids := []string{}
	for n := 1; n < 9; n++ {
		ids = append(ids, fmt.Sprintf("claude-%d", n))
	}
	basicsEqual(t, basicsSet(wakeField(found, "id")...), basicsSet(ids...))
	basicsEqual(t, basicsSet(wakeField(found, "secret")...), basicsSet(strings.Fields(wakeSet)...))
	basicsEqual(t, basicsSet(wakeField(found, "home")...), basicsSet(".harness"))
	basicsEqual(t, notes, []string{})
}

func TestWake_ARunningSessionAMissingSecretAndASuspensionAreLeftAlone(t *testing.T) {
	running := pyjson.New("usage", pyjson.New("five_hour", pyjson.New("utilization", 0.3,
		"resets_at", clock.ISO(wakeCDTSix.Add(2*time.Hour)))))
	over := pyjson.New("usage", pyjson.New("five_hour", pyjson.New("utilization", 0.9,
		"resets_at", clock.ISO(wakeCDTSix.Add(-5*time.Minute)))))
	state := pyjson.New("providers", pyjson.New("claude-1", running, "claude-2", over,
		"claude-3", pyjson.New("suspended", pyjson.New("by", "jgoetzmann"))))
	found, notes := wake.Accounts(wakeCfg(strings.ReplaceAll(wakeSet, "CLAUDE_CODE_OAUTH_TOKEN_4", "")),
		state, wakeCDTSix, ".harness")
	ids := wakeField(found, "id")
	basicsNotIn(t, "claude-1", ids)
	basicsIn(t, "claude-2", ids, "a session whose window has ended is not running")
	basicsNotIn(t, "claude-3", ids)
	basicsNotIn(t, "claude-4", ids)
	basicsEqual(t, len(notes), 3, notes)
}

func TestWake_ASessionItsLimitRefusesIsLeftAloneUntilItResets(t *testing.T) {
	refused := pyjson.New("refused_until", clock.ISO(wakeCDTSix.Add(3*time.Hour)))
	week := pyjson.New("usage", pyjson.New(
		"five_hour", pyjson.New("utilization", 0.4, "resets_at", clock.ISO(wakeCDTSix.Add(-time.Hour))),
		"seven_day", pyjson.New("utilization", 1.0, "resets_at", clock.ISO(wakeCDTSix.Add(48*time.Hour)))))
	over := pyjson.New("refused_until", clock.ISO(wakeCDTSix.Add(-5*time.Minute)),
		"usage", pyjson.New("seven_day", pyjson.New("utilization", 1.0,
			"resets_at", clock.ISO(wakeCDTSix.Add(-5*time.Minute)))))
	state := pyjson.New("providers", pyjson.New("claude-1", refused, "claude-2", week, "claude-3", over))
	found, notes := wake.Accounts(wakeCfg(wakeSet), state, wakeCDTSix, ".harness")
	ids := wakeField(found, "id")
	basicsNotIn(t, "claude-1", ids)
	basicsNotIn(t, "claude-2", ids)
	basicsIn(t, "claude-3", ids, "a refusal that has run out holds nothing")
	basicsEqual(t, len(notes), 2, notes)
	all := true
	for _, n := range notes {
		all = all && strings.Contains(n, "usage limit refuses calls until")
	}
	basicsTrue(t, all, notes)
}

// The list is the pool's Claude subscriptions, however many there are.
func TestWake_EveryNewClaudeSubscriptionIsWokenAndNoneOfAnotherCLI(t *testing.T) {
	cfg := wakeCfg(wakeSet)
	claude := []string{}
	for _, p := range cfg.Pool.Ordered() {
		if p.CLI == "claude" && p.Login == "secret" && p.Enabled {
			claude = append(claude, p.ID)
		}
	}
	found, _ := wake.Accounts(cfg, pyjson.New("providers", pyjson.New()), wakeCDTSix, ".harness")
	basicsEqual(t, wakeField(found, "id"), claude)
}

func TestWake_AHaltedBotWakesNothing(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := testsupport.MakeCtx(gh, testsupport.Ctx{At: wakeCDTSix, Cfg: wakeCfg(wakeSet)})
	found, _ := wake.ToWake(ctx, ".harness")
	basicsEqual(t, len(found), 8)
	ctx.Store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "halt")
	found, notes := wake.ToWake(ctx, ".harness")
	basicsEqual(t, found, []*pyjson.Object{})
	basicsEqual(t, notes, []string{"halted: a wake is a model call"})
}

// wakeRan is ran(): a stand-in for `subprocess.run` whose CLI prints `stdout`; `seen` (when not
// nil) keeps the argv it was given.
func wakeRan(stdout string, seen map[string]any) runner.RunFunc {
	return func(argv []string, args runner.RunArgs) (*runner.Completed, error) {
		if seen != nil {
			seen["argv"] = argv
		}
		return &runner.Completed{Stdout: stdout}, nil
	}
}

// wakeEvent is event(): a rate-limit event of the stream, `info` added to its rate_limit_info.
func wakeEvent(status string, fiveHour float64, info ...any) string {
	limit := pyjson.New("status", status, "unifiedWindows", pyjson.New(
		"five_hour", pyjson.New("utilization", fiveHour, "resetsAt", 1790710800)))
	limit.Update(pyjson.New(info...))
	return pyjson.Dumps(pyjson.New("type", "rate_limit_event", "rate_limit_info", limit))
}

// wakeAfter is argv[argv.index(flag) + 1].
func wakeAfter(argv []string, flag string) string {
	return argv[slices.Index(argv, flag)+1]
}

// ------------------------------------------------------------------ PingTests

func TestWake_TheWakeIsOneTurnOfHaikuAtLowEffortWithNothingLoaded(t *testing.T) {
	seen := map[string]any{}
	found := runner.PingUsage("claude", wake.Model, wakeRan(wakeEvent("allowed", 0.01), seen), "", wake.Effort)
	basicsAlmostEqual(t, found.Obj("five_hour").Float("utilization"), 0.01)
	argv := seen["argv"].([]string)
	basicsEqual(t, wakeAfter(argv, "--model"), "claude-haiku-5-5")
	basicsEqual(t, wakeAfter(argv, "--effort"), "low")
	basicsEqual(t, wakeAfter(argv, "--max-turns"), "1")
	basicsEqual(t, wakeAfter(argv, "--tools"), "")
	basicsEqual(t, wakeAfter(argv, "--system-prompt"), runner.PingSystem)
	for _, flag := range []string{"--strict-mcp-config", "--disable-slash-commands", "--no-session-persistence"} {
		basicsIn(t, flag, argv)
	}
}

func TestWake_TheQuietPingNamesNoEffort(t *testing.T) {
	seen := map[string]any{}
	runner.PingUsage("claude", "haiku", wakeRan("", seen), "", "")
	basicsNotIn(t, "--effort", seen["argv"].([]string))
}

// ------------------------------------------------------------------ WakeTests

// wakeWake is WakeTests.wake: wake `claude-1` (of a config with every Claude secret set) with the
// CLI printing `stdout`.
func wakeWake(stdout string) (bool, string) {
	cfg := wakeCfg(wakeSet)
	real := wake.Ping
	wake.Ping = func(claudeBin, model string, run runner.RunFunc, token, effort string) (*pyjson.Object, *pyjson.Object) {
		return real(claudeBin, model, wakeRan(stdout, nil), token, effort)
	}
	defer func() { wake.Ping = real }()
	return wake.Wake(cfg, "claude-1")
}

func TestWake_APingStartsTheSession(t *testing.T) {
	ok, line := wakeWake(wakeEvent("allowed", 0.01))
	basicsTrue(t, ok)
	basicsIn(t, "the 5-hour session is running", line)
}

func TestWake_APingItsLimitRejectsIsReportedNotFailed(t *testing.T) {
	ok, line := wakeWake(strings.Join([]string{
		wakeEvent("rejected", 1.0, "resetsAt", 1790710800),
		pyjson.Dumps(pyjson.New("type", "result", "is_error", true, "result", "You've hit your limit"))}, "\n"))
	basicsTrue(t, ok, line)
	basicsIn(t, "its usage limit refused the ping", line)
	basicsIn(t, clock.ISO(time.Unix(1790710800, 0).UTC()), line)
}

func TestWake_ARefusalTheCLIGivesOnlyInWordsIsOneToo(t *testing.T) {
	ok, line := wakeWake(pyjson.Dumps(pyjson.New("type", "result", "is_error", true,
		"result", "Claude AI usage limit reached")))
	basicsTrue(t, ok, line)
	basicsIn(t, "its usage limit refused the ping", line)
}

func TestWake_APingThatSaysNothingFails(t *testing.T) {
	ok, line := wakeWake(pyjson.Dumps(pyjson.New("type", "result", "is_error", true,
		"result", "connection reset")))
	basicsFalse(t, ok)
	basicsIn(t, "no usage reading", line)
}
