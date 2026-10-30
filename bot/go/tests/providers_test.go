package tests

// bot/tests/test_providers.py: subscriptions: `.harness/providers.json`, availability, and which
// one takes which item.

import (
	"os"
	"path/filepath"
	"reflect"
	"strconv"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cli"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/plan"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/status"
	ts "github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// ---------------------------------------------------------------------------- the file's helpers

// providersAll = " ".join(providers.SECRETS)
var providersAll = strings.Join(providers.SecretNames, " ")

// providersZone is the bot's time zone, as the tests name it.
const providersZone = "America/Chicago"

// providersRaw is raw_providers(): `.harness/providers.json`'s JSON.
func providersRaw() *pyjson.Object {
	return providersLoad(filepath.Join(ts.Root, providers.ProvidersPath))
}

// providersLoad is json.loads(path.read_text(encoding="utf-8")).
func providersLoad(path string) *pyjson.Object {
	text, err := os.ReadFile(path)
	if err != nil {
		panic(err)
	}
	v, err := pyjson.Loads(text)
	if err != nil {
		panic(err)
	}
	return v.(*pyjson.Object)
}

// providersOf is raw["providers"][name].
func providersOf(raw *pyjson.Object, name string) *pyjson.Object {
	return raw.Obj("providers").Obj(name)
}

// providersPoolWith is pool_with(**changes): each provider's fields updated, in order.
func providersPoolWith(name string, fields *pyjson.Object) *providers.Pool {
	raw := providersRaw()
	providersOf(raw, name).Update(fields)
	return providers.Parse(raw)
}

// providersSecrets is secrets(*names).
func providersSecrets(names ...string) map[string]string {
	return map[string]string{"HARNESS_SECRETS_SET": strings.Join(names, " ")}
}

// providersEveryone is Secrets.of(secrets(*providers.SECRETS)).
func providersEveryone() *providers.Secrets {
	return providers.SecretsOf(providersSecrets(providers.SecretNames...))
}

// providersReading is {"utilization": used, "resets_at": resets}.
func providersReading(used float64, resets string) *pyjson.Object {
	return pyjson.New("utilization", used, "resets_at", resets)
}

// providersState is {"providers": {id: entry}}.
func providersState(id string, entry *pyjson.Object) *pyjson.Object {
	return pyjson.New("providers", pyjson.New(id, entry))
}

// providersStr is `str` of a value a test reads from a plan or a thread.
func providersStr(v any) string { s, _ := v.(string); return s }

// providersCtx is ctx_for's keyword arguments; a zero field is its default. Machine nil is
// MACHINE; an empty (non-nil) slice is machine=().
type providersCtx struct {
	At             time.Time // = NIGHT
	Env            map[string]string
	Machine        []string
	CommittedHours bool
}

// providersCtxFor is ctx_for(gh, ...).
func providersCtxFor(gh *fakes.FakeGitHub, opts ...providersCtx) *hctx.Context {
	var o providersCtx
	if len(opts) > 0 {
		o = opts[0]
	}
	at := o.At
	if at.IsZero() {
		at = ts.Night
	}
	env := o.Env
	if len(env) == 0 {
		env = providersSecrets(providers.SecretNames...)
	}
	machine := o.Machine
	if machine == nil {
		machine = ts.Machine
	}
	return ts.MakeCtx(gh, ts.Ctx{At: at, Cfg: ts.MakeConfig(ts.Config{Env: env, Machine: machine,
		CommittedHours: o.CommittedHours})})
}

// providersForce is plan_mod.make(ctx, force=True).
func providersForce(ctx *hctx.Context) *pyjson.Object {
	return plan.Make(ctx, true, nil, "auto", nil)
}

// providersRun is gh.runs[id] = {"status": status}.
func providersRun(gh *fakes.FakeGitHub, id, status string) {
	gh.Runs.Set(id, pyjson.New("status", status))
}

// providersPicked is (planned["number"], planned["provider"], planned["action"]).
func providersPicked(planned *pyjson.Object) []any {
	return []any{planned.Get("number"), planned.Get("provider"), planned.Get("action")}
}

// providersWithMaxParallel is dataclasses.replace(cfg, pool=dataclasses.replace(cfg.pool,
// max_parallel=n)).
func providersWithPool(cfg *config.Config, change func(*providers.Pool)) *config.Config {
	pool := *cfg.Pool
	change(&pool)
	replaced := *cfg
	replaced.Pool = &pool
	return &replaced
}

// ---------------------------------------------------------------------------- ParseTests

// #1455: the organization runs 20 hosted jobs at once, and CI takes the rest. The bots share one
// pool of 8, first come first served: each bot's `max_parallel` is the whole pool, and each counts
// the other's runs against it (`plan.others_on_github`), planning runs included
// (`plan.Lanes.free`).
func TestProviders_TheBotsShareEightOfGithubsRunners(t *testing.T) {
	night := providersLoad(filepath.Join(ts.Root, ".harness", "providers.json"))
	squishy := providersLoad(filepath.Join(ts.Root, ".squishy", "providers.json"))
	assertEqual(t, night.Get("max_parallel"), squishy.Get("max_parallel"))
	assertTrue(t, night.Int("max_parallel") <= 8)
	for _, pair := range [][2]string{{".harness", "squishy-run.yml"}, {".squishy", "bot-night.yml"}} {
		raw := providersLoad(filepath.Join(ts.Root, pair[0], "config.json"))
		workflows := []any{}
		for _, o := range pyjson.List(raw.Obj("identity").Get("others")) {
			workflows = append(workflows, pyjson.Obj(o).Get("workflow"))
		}
		assertEqual(t, workflows, []any{pair[1]})
	}
}

func TestProviders_TheCommittedFile(t *testing.T) {
	pool := providers.Load(ts.Root)
	// The usage order: claude-3 (no caps), claude-7, claude-8 and claude-1 first, then
	// claude-4, claude-6 and claude-5 last of the Claude accounts (each under its caps), then the
	// medium models, Muse first (#317 part 9), then claude-2, kept for planning and review, last.
	// (The AI's training lanes run outside the harness: no subscription of theirs.)
	assertEqual(t, pool.Priority, []string{"claude-3", "claude-7", "claude-8", "claude-1",
		"claude-4", "claude-6", "claude-5", "muse", "agy", "gpt", "claude-2"})
	assertEqual(t, []any{pool.MaxParallel, pool.MachineParallel}, []any{8, 6})
	assertTrue(t, pool.Balance) // the load balancer (`plan.balance`)
	clis := map[string]bool{}
	claude := 0
	for _, p := range pool.Ordered() {
		clis[p.CLI] = true
		if p.CLI == "claude" {
			claude++
		}
	}
	assertEqual(t, clis, setOf(providers.Clis...))
	assertEqual(t, claude, 8)
	first := pool.Get("claude-1")
	assertEqual(t, first.Secret, "CLAUDE_CODE_OAUTH_TOKEN")
	// No subscription waits for quiet any more: the bot spends claude-1 like the rest.
	for _, p := range pool.Ordered() {
		assertFalse(t, p.QuietCheck, p.ID)
	}
	seats := map[string][]any{}
	for _, p := range pool.Ordered() {
		list := []any{}
		for _, s := range pool.Seats(p) {
			list = append(list, []any{s.Model, s.Tier, s.SelfCheck})
		}
		seats[p.ID] = list
	}
	both := func() []any {
		return []any{[]any{"opus", "strong", false}, []any{"sonnet", "medium", false}}
	}
	want := map[string][]any{
		"claude-3": both(), "claude-1": both(), "claude-4": both(), "claude-5": both(),
		"claude-6": both(), "claude-7": both(), "claude-8": both(),
		"agy":      {[]any{"gemini-3.8-flash-high", "medium", false}},
		"muse":     {[]any{"muse-spark-1.3-contributor", "medium", false}},
		"gpt":      {[]any{"gpt-5.6-sol", "medium", false}},
		"claude-2": both(),
	}
	assertEqual(t, len(seats), len(want))
	for id, list := range want {
		assertEqual(t, seats[id], list, id)
	}
	// No committed subscription is weak since #2129: an easy item goes to the medium models
	// first. The tests hold the weak tier's rules to one of their own (`support.WEAK`).
	weak, ok := pool.Tiers.Lookup("weak")
	assertTrue(t, ok, "tiers has no weak")
	assertEqual(t, len(weak.([]*providers.TierEntry)), 0)
	models := func(tier string) []string {
		out := []string{}
		for _, e := range pool.Tiers.Get(tier).([]*providers.TierEntry) {
			out = append(out, e.Model)
		}
		return out
	}
	assertEqual(t, models("medium"), []string{"muse-spark-1.3-contributor", "gemini-3.8-flash-high",
		"gpt-5.6-sol", "sonnet"})
	// Every Claude account switches between its Opus and its Sonnet as the work needs: no seat on
	// them only stands in for another subscription.
	takesOver := map[string][]string{}
	for _, p := range pool.Ordered() {
		if p.CLI != "claude" {
			continue
		}
		list := []string{}
		for _, s := range pool.Seats(p) {
			list = append(list, s.TakesOver)
		}
		takesOver[p.ID] = list
	}
	wantOver := map[string][]string{}
	for _, name := range []string{"claude-1", "claude-2", "claude-3", "claude-4", "claude-5",
		"claude-6", "claude-7", "claude-8"} {
		wantOver[name] = []string{"", ""}
	}
	assertEqual(t, takesOver, wantOver)
	assertEqual(t, models("strong"), []string{"opus"})
	kind := reflect.TypeOf(providers.Provider{})
	for range pool.Ordered() {
		_, difficult := kind.FieldByName("Difficult")
		_, selfReview := kind.FieldByName("SelfReview")
		assertFalse(t, difficult || selfReview)
	}
	assertNotIn(t, "difficult_label", providersRaw())
}

func TestProviders_ParseTests_MistakesAreErrors(t *testing.T) {
	broken := func(change func(r *pyjson.Object)) {
		t.Helper()
		raw := providersRaw()
		change(raw)
		assertRaises[*herr.ConfigError](t, func() { providers.Parse(raw) })
	}
	gpt := func(fields ...any) func(r *pyjson.Object) {
		return func(r *pyjson.Object) { providersOf(r, "gpt").Update(pyjson.New(fields...)) }
	}
	broken(gpt("surprise", 1))
	broken(gpt("cli", "copilot"))
	broken(gpt("secret", "BOT_GITHUB_TOKEN"))
	broken(gpt("secret", "CLAUDE_CODE_OAUTH_TOKEN"))
	broken(gpt("limits", pyjson.New("mode", "caps", "five_hour", 98)))
	broken(gpt("limits", pyjson.New("mode", "caps")))
	broken(gpt("only_labels", []any{"training"})) // retired
	broken(gpt("schedule", pyjson.New("mode", "window", "start", "9")))
	broken(gpt("roles", []any{"build", "dance"}))
	broken(func(r *pyjson.Object) { r.Set("priority", []any{"claude-1"}) })
	broken(func(r *pyjson.Object) { r.Set("max_parallel", 0) })
	broken(func(r *pyjson.Object) { r.Set("balance", "yes") })
}

func TestProviders_TierMistakesAreErrors(t *testing.T) {
	broken := func(change func(r *pyjson.Object), words string) {
		t.Helper()
		raw := providersRaw()
		change(raw)
		assertRaisesRegex[*herr.ConfigError](t, words, func() { providers.Parse(raw) })
	}
	update := func(name string, fields ...any) func(r *pyjson.Object) {
		return func(r *pyjson.Object) { providersOf(r, name).Update(pyjson.New(fields...)) }
	}
	broken(update("gpt", "tier", "genius"), "not one of weak")
	broken(func(r *pyjson.Object) { providersOf(r, "gpt").Delete("tier") }, "missing tier")
	broken(update("gpt", "tier", "strong"), "tiers lists it under medium")
	broken(update("gpt", "model", "gpt-9"), "does not list it")
	broken(func(r *pyjson.Object) {
		tiers := r.Obj("tiers")
		tiers.Set("medium", append(tiers.List("medium"), pyjson.New("model", "opus")))
	}, "more than one place")
	broken(func(r *pyjson.Object) { r.Obj("tiers").Set("huge", []any{}) }, "unknown tiers")
	broken(func(r *pyjson.Object) { r.Delete("tiers") }, "expected an object")
	broken(update("claude-2", "extra_models", []any{pyjson.New("model", "sonnet", "tier", "weak")}),
		"tiers lists it under medium")
	broken(update("claude-2", "extra_models", []any{pyjson.New("model", "sonnet", "tier", "medium",
		"takes_over", "nobody")}), "which is not another provider")
	broken(update("claude-2", "extra_models", []any{pyjson.New("tier", "weak")}), "needs a model")
	broken(update("gpt", "difficult", true), "unknown keys difficult")
	broken(update("gpt", "self_review", true), "unknown keys self_review")
}

func TestProviders_LoginsAndRunners(t *testing.T) {
	pool := providers.Load(ts.Root)
	for _, provider := range pool.Ordered() {
		// The Claude accounts run on GitHub's runners; every other subscription has a runner of
		// its own on the machine (bot/machine/README.md).
		claude := provider.CLI == "claude"
		runsOn, login := "night-vm-"+provider.ID, "machine"
		if claude {
			runsOn, login = "ubuntu-latest", "secret"
		}
		assertEqual(t, provider.RunsOn, runsOn)
		assertEqual(t, provider.Login, login)
		assertEqual(t, pool.OnMachine(provider.ID), !claude)
	}
	assertEqual(t, pool.Get("gpt").Secret, "")

	broken := func(change func(r *pyjson.Object), words string) {
		t.Helper()
		raw := providersRaw()
		change(raw)
		assertRaisesRegex[*herr.ConfigError](t, words, func() { providers.Parse(raw) })
	}
	update := func(name string, fields ...any) func(r *pyjson.Object) {
		return func(r *pyjson.Object) { providersOf(r, name).Update(pyjson.New(fields...)) }
	}
	broken(update("gpt", "secret", "CODEX_AUTH_JSON"), "takes no secret")
	broken(update("gpt", "runs_on", "ubuntu-latest"), "its runner")
	broken(update("gpt", "runs_on", "night vm"), "not a runner label")
	broken(update("gpt", "login", "keyring"), "not one of")
	broken(update("agy", "login", "secret", "secret", "MUSE_AUTH", "runs_on", "ubuntu-latest"),
		"on the machine only")
	broken(update("muse", "runs_on", "night-vm-gpt"), "share the runner")
	broken(update("claude-2", "runs_on", "night-vm-gpt"), "share the runner")
	broken(update("gpt", "lanes", 0), "at least 1")
	broken(update("gpt", "lanes", "two"), "not a number")
	broken(func(r *pyjson.Object) { r.Set("machine_parallel", -1) }, "machine_parallel")
	// The machine's slots are its own, apart from `max_parallel`: it may have more.
	assertEqual(t, providers.Parse(providersRaw().Set("max_parallel", 2)).MachineParallel, 6)
	// A secret login may still run on GitHub's runners, and those are shared by design.
	raw := providersRaw()
	for _, name := range []string{"claude-1", "claude-2"} {
		providersOf(raw, name).Set("runs_on", "ubuntu-latest")
	}
	providersOf(raw, "gpt").Update(pyjson.New("login", "secret", "secret", "CODEX_AUTH_JSON",
		"runs_on", "ubuntu-latest"))
	assertEqual(t, providers.Parse(raw).Get("gpt").Secret, "CODEX_AUTH_JSON")
}

// ---------------------------------------------------------------------------- AvailabilityTests

// providersWhyArgs is AvailabilityTests.why's keyword arguments; a zero field is its default.
type providersWhyArgs struct {
	State  *pyjson.Object // = None: {"providers": {}} (and so is {})
	At     time.Time      // = NIGHT
	Env    map[string]string
	Forced bool
	Pool   *providers.Pool // = None: providers.load(ROOT)
}

// providersWhy is AvailabilityTests.why.
func providersWhy(providerID string, opts ...providersWhyArgs) string {
	var o providersWhyArgs
	if len(opts) > 0 {
		o = opts[0]
	}
	pool := o.Pool
	if pool == nil {
		pool = providers.Load(ts.Root)
	}
	st := o.State
	if !pyjson.Truthy(st) {
		st = pyjson.New("providers", pyjson.New())
	}
	at := o.At
	if at.IsZero() {
		at = ts.Night
	}
	env := o.Env
	if len(env) == 0 {
		env = providersSecrets(providers.SecretNames...)
	}
	return providers.Availability(pool.Get(providerID), st, at, providersZone,
		providers.SecretsOf(env), o.Forced, false)
}

// `usage_since`: Muse's last reading (2026-10-09, the $15 plan) said 100% of its week, resetting
// on 2026-10-12. On the $50 plan since 2026-10-11 that reading holds nothing back, and the next
// run reads the new plan's usage; a reading taken since still counts.
func TestProviders_AReadingFromBeforeAPlanChangeHoldsNothing(t *testing.T) {
	old := func() *pyjson.Object {
		return pyjson.New("observed_at", "2026-10-09T11:09:34Z", "status", "allowed",
			"seven_day", pyjson.New("resets_at", "2026-10-12T00:00:00Z", "utilization", 1.0))
	}
	tonight := time.Date(2026, 10, 11, 22, 0, 0, 0, time.UTC)
	assertEqual(t, providers.Load(ts.Root).Get("muse").UsageSince.Format("2006-01-02 15:04:05-07:00"),
		"2026-10-11 00:00:00+00:00")
	assertIsNone(t, providersWhy("muse", providersWhyArgs{
		State: providersState("muse", pyjson.New("usage", old())), At: tonight}))
	// Without `usage_since`, the same reading holds it until the old plan's reset.
	unchanged := providersPoolWith("muse", pyjson.New("usage_since", nil))
	assertIn(t, "7-day usage is 100%", providersWhy("muse", providersWhyArgs{
		State: providersState("muse", pyjson.New("usage", old())), At: tonight, Pool: unchanged}))
	fresh := old().Set("observed_at", "2026-10-11T21:30:00Z")
	assertIn(t, "7-day usage is 100%", providersWhy("muse", providersWhyArgs{
		State: providersState("muse", pyjson.New("usage", fresh)), At: tonight}))
	assertRaisesRegex[*herr.ConfigError](t, "is not a time", func() {
		providersPoolWith("muse", pyjson.New("usage_since", "last Tuesday"))
	})
}

func TestProviders_HoursSecretsAndTheSwitch(t *testing.T) {
	// claude-4 works any hour: up to 70% in its 03:00–15:00 window, under 50% outside it.
	assertIsNone(t, providersWhy("claude-4"))
	assertIsNone(t, providersWhy("claude-4", providersWhyArgs{At: ts.Day}))
	// A window with no `off_hours` shuts outside its hours, unless forced.
	raw := providersRaw()
	providersOf(raw, "claude-4").Delete("off_hours")
	windowed := providers.Parse(raw)
	assertIn(t, "outside its hours (03:00–15:00", providersWhy("claude-4", providersWhyArgs{Pool: windowed}))
	assertIsNone(t, providersWhy("claude-4", providersWhyArgs{Forced: true, Pool: windowed}))
	assertIsNone(t, providersWhy("claude-4", providersWhyArgs{At: ts.Day, Pool: windowed}))
	assertIsNone(t, providersWhy("claude-1", providersWhyArgs{At: ts.Day})) // any hour, under its caps
	assertIsNone(t, providersWhy("gpt", providersWhyArgs{At: ts.Day}))
	assertIn(t, "`CLAUDE_CODE_OAUTH_TOKEN_2` is not set",
		providersWhy("claude-2", providersWhyArgs{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN")}))
	// A login on the machine has no secret to be missing.
	assertIsNone(t, providersWhy("gpt", providersWhyArgs{Env: providersSecrets()}))
	keyed := providersPoolWith("gpt", pyjson.New("login", "secret", "secret", "CODEX_AUTH_JSON",
		"runs_on", "ubuntu-latest"))
	assertIn(t, "`CODEX_AUTH_JSON` is not set",
		providersWhy("gpt", providersWhyArgs{Env: providersSecrets("MUSE_AUTH"), Pool: keyed}))
	off := providersPoolWith("gpt", pyjson.New("enabled", false))
	assertIn(t, "switched off", providersWhy("gpt", providersWhyArgs{Pool: off}))
	// Run by hand, nothing says which secrets the workflow has: nothing is ruled out.
	assertIsNone(t, providers.Availability(providers.Load(ts.Root).Get("gpt"), pyjson.New(), ts.Day,
		providersZone, providers.SecretsOf(map[string]string{}), false, false))
}

func TestProviders_CapsRefusalsAndMinutes(t *testing.T) {
	later := clock.ISO(ts.Night.Add(2 * time.Hour))
	st := providersState("claude-2", pyjson.New("usage", pyjson.New("seven_day", providersReading(0.95, later))))
	assertIn(t, "7-day usage is 95%", providersWhy("claude-2", providersWhyArgs{State: st}))
	// --force lifts a subscription's hours, never its caps.
	assertIn(t, "7-day usage is 95%", providersWhy("claude-2", providersWhyArgs{State: st, At: ts.Day,
		Forced: true}))
	// claude-2 stops at 90% of its 5-hour session too; claude-1 goes on to 100%.
	session := func() *pyjson.Object { return pyjson.New("five_hour", providersReading(0.92, later)) }
	assertIn(t, "5-hour usage is 92%", providersWhy("claude-2", providersWhyArgs{
		State: providersState("claude-2", pyjson.New("usage", session()))}))
	assertIsNone(t, providersWhy("claude-1", providersWhyArgs{
		State: providersState("claude-1", pyjson.New("usage", session()))}))
	assertIsNone(t, providersWhy("claude-2", providersWhyArgs{State: st, At: ts.Night.Add(3 * time.Hour)}))
	// `none` has no caps: only a refusal stops it, until its reset.
	st = providersState("gpt", pyjson.New("usage", pyjson.New("seven_day", providersReading(0.99, later))))
	assertIsNone(t, providersWhy("gpt", providersWhyArgs{State: st}))
	s := pyjson.New("providers", pyjson.New())
	providers.NoteUsage(s, "gpt", nil, "+PT30M", ts.Night, 0, nil, nil)
	assertIn(t, "refused a call", providersWhy("gpt", providersWhyArgs{State: s}))
	assertIsNone(t, providersWhy("gpt", providersWhyArgs{State: s, At: ts.Night.Add(31 * time.Minute)}))
	budget := providersPoolWith("muse", pyjson.New("limits", pyjson.New("mode", "caps", "seven_day_minutes", 60)))
	s = pyjson.New("providers", pyjson.New())
	providers.NoteUsage(s, "muse", nil, "", ts.Night.Add(-2*24*time.Hour), 40, nil, nil)
	providers.NoteUsage(s, "muse", nil, "", ts.Night.Add(-time.Hour), 25, nil, nil)
	assertIn(t, "spent 65 of its 60 minutes", providersWhy("muse", providersWhyArgs{State: s, Pool: budget}))
	providers.NoteUsage(s, "muse", nil, "", ts.Night.Add(-9*24*time.Hour), 999, nil, nil)
	// a week-old run has gone
	assertIn(t, "spent 65", providersWhy("muse", providersWhyArgs{State: s, Pool: budget}))
}

// `room`, the load balancer's reading: the tightest window's share of its cap that the last
// reading leaves free (a window with no cap has the whole allowance), and of its minute budgets; a
// window reset since the reading is all free, a refusal leaves none.
func TestProviders_RoomIsTheShareOfItsAllowanceLeft(t *testing.T) {
	pool := providers.Load(ts.Root)
	later := clock.ISO(ts.Night.Add(24 * time.Hour))

	type window struct {
		name string
		used float64
	}
	room := func(providerID string, at time.Time, refused string, windows ...window) float64 {
		if at.IsZero() {
			at = ts.Night
		}
		usage := pyjson.New()
		for _, w := range windows {
			usage.Set(w.name, providersReading(w.used, later))
		}
		entry := pyjson.New("usage", usage)
		if refused != "" {
			entry.Set("refused_until", refused)
		}
		return providers.Room(pool.Get(providerID), entry, at, providersZone)
	}
	five := func(used float64) window { return window{"five_hour", used} }
	seven := func(used float64) window { return window{"seven_day", used} }

	assertEqual(t, room("claude-5", time.Time{}, ""), 1.0) // no reading
	// claude-5's caps are 40% of its 5-hour session and 60% of its week.
	assertAlmostEqual(t, room("claude-5", time.Time{}, "", five(0.2), seven(0.3)), 0.5)
	assertAlmostEqual(t, room("claude-5", time.Time{}, "", five(0.1), seven(0.45)), 0.25)
	assertEqual(t, room("claude-5", time.Time{}, "", five(0.5)), 0.0) // past its cap
	assertEqual(t, room("claude-5", ts.Night.Add(2*24*time.Hour), "", five(0.5)), 1.0)
	assertEqual(t, room("claude-5", time.Time{}, later), 0.0)
	// claude-3 has no caps: its readings count against the whole allowance.
	assertAlmostEqual(t, room("claude-3", time.Time{}, "", seven(0.7)), 0.3)
	// claude-7: 70% of its session in its 01:00–10:00 window, 50% outside it.
	morning := ts.Day.Add(-(2*time.Hour + 30*time.Minute)) // 09:30 CDT
	assertAlmostEqual(t, room("claude-7", morning, "", five(0.35)), 0.5)
	assertAlmostEqual(t, room("claude-7", ts.Day, "", five(0.35)), 0.3)
	budget := providersPoolWith("muse", pyjson.New("limits", pyjson.New("mode", "caps", "seven_day_minutes", 60)))
	entry := pyjson.New("providers", pyjson.New())
	providers.NoteUsage(entry, "muse", nil, "", ts.Night.Add(-time.Hour), 45, nil, nil)
	assertAlmostEqual(t, providers.Room(budget.Get("muse"), entry.Obj("providers").Obj("muse"), ts.Night,
		providersZone), 0.25)
}

func TestProviders_TheOldTopLevelReadingsBelongToTheFirstClaudeAccount(t *testing.T) {
	later := clock.ISO(ts.Night.Add(2 * time.Hour))
	st := pyjson.New("usage", pyjson.New("five_hour", providersReading(1.0, later)), "providers", pyjson.New())
	assertIn(t, "5-hour usage is 100%", providersWhy("claude-1", providersWhyArgs{State: st}))
	assertIsNone(t, providersWhy("claude-2", providersWhyArgs{State: st}))
	providers.NoteUsage(st, "claude-1", pyjson.New("five_hour", providersReading(0.1, later)), "",
		ts.Night, 0, nil, nil)
	assertIsNone(t, providersWhy("claude-1", providersWhyArgs{State: st}))
}

// ---------------------------------------------------------------------------- MatchingTests
//
// `plan.make` with several subscriptions: who takes what.

func TestProviders_ByDayTheFirstFreeSubscriptionWithItsSecretBuilds(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned := flowMake(providersCtxFor(gh, providersCtx{At: ts.Day}))
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider"), planned.Get("cli"),
		planned.Get("secret"), planned.Get("login"), planned.Get("runs_on")},
		[]any{3, "muse", "muse", "", "machine", "night-vm-muse"})
	gh2 := fakes.NewFakeGitHub()
	gh2.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned = flowMake(providersCtxFor(gh2, providersCtx{At: ts.Day, Machine: []string{"agy"}}))
	assertEqual(t, []any{planned.Get("provider"), planned.Get("runs_on")}, []any{"agy", "night-vm-agy"})
	assertEqual(t, providersCtxFor(gh2).Store.Load().Obj("items").Obj("3").Get("provider"), "agy")
}

// The tests' weak subscription (`support.WEAK`, as the weak seat was until #2129) logs in on the
// machine, works any hour with no caps, is weak and checks its own builds. It is last in the usage
// order, but an easy item goes to it first while it has a free lane (`easy_first`), planned by a
// free medium or strong model on its own run.
func TestProviders_AWeakSubscriptionTakesEasyItemsFirstAndNothingHarder(t *testing.T) {
	pool := ts.MakeConfig(ts.Config{Machine: ts.AllMachine}).Pool
	lite := pool.Get(ts.Weak)
	assertEqual(t, []any{lite.CLI, lite.Family, lite.Login, lite.RunsOn},
		[]any{"codex", "lite", "machine", "night-vm-lite"})
	assertEqual(t, []any{lite.Schedule.Mode, lite.Limits.Mode}, []any{"always", "none"})
	assertEqual(t, []any{lite.Tier, lite.SelfCheck, lite.EasyFirst}, []any{"weak", true, true})
	assertEqual(t, pool.Priority[len(pool.Priority)-1], ts.Weak)
	// Unplanned, the weak one cannot take it: it builds only from a plan. By day no strong model
	// is free (the Claude accounts keep to the night here), so Muse plans it in its own run and
	// builds it.
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"}})
	planned := flowMake(providersCtxFor(gh, providersCtx{At: ts.Day, Machine: ts.AllMachine}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	assertIn(t, "#3 needs a plan", planned.Get("housekeeping"))
	// A medium item passes the weak one by.
	gh = fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned = flowMake(providersCtxFor(gh, providersCtx{At: ts.Day, Machine: ts.AllMachine}))
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "muse"})
	// With nothing else free, the weak one builds an easy item another model already planned.
	gh = fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"}})
	ctx := providersCtxFor(gh, providersCtx{At: ts.Day, Machine: []string{ts.Weak}})
	assertEqual(t, flowMake(ctx).Get("action"), "none") // no planner is free
	// A medium model's plan is enough for an easy item (#317 part 6), and the weak one builds it.
	flowUpdateItem(ctx, 3, pyjson.New("planned_at", clock.ISO(ts.Day), "planned_tier", "medium"))
	planned = flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", ts.Weak})
	assertEqual(t, planned.Obj("seats").Get("self_check"), true)
	assertEqual(t, planned.Obj("seats").Get("review"), nil) // weak reviews only in a review run
}

// `off_from`: from that day (Central time) a subscription takes no new work, forced or not, and
// the planner opens one issue asking a person what it should do now.
func TestProviders_ASubscriptionIsSwitchedOffFromItsOffFromDay(t *testing.T) {
	cfg := ts.MakeConfig(ts.Config{Env: providersSecrets(providers.SecretNames...), Machine: ts.AllMachine})
	lite := *cfg.Pool.Get(ts.Weak)
	lite.OffFrom = time.Date(2026, 11, 2, 0, 0, 0, 0, time.UTC)
	lite.OffReason = "Its plan runs out on 2026-11-02."
	cfg = providersWithPool(cfg, func(pool *providers.Pool) {
		pool.Providers = pool.Providers.Copy().Set(ts.Weak, &lite)
	})
	everyone := providersEveryone()
	lateOnThe1st := time.Date(2026, 11, 2, 5, 59, 0, 0, time.UTC) // 23:59 CST
	assertIsNone(t, providers.Availability(&lite, pyjson.New(), lateOnThe1st, providersZone, everyone,
		false, false))
	onThe2nd := time.Date(2026, 11, 2, 6, 0, 0, 0, time.UTC)
	for _, forced := range []bool{false, true} {
		assertIn(t, "switched off from 2026-11-02", providers.Availability(&lite, pyjson.New(),
			onThe2nd, providersZone, everyone, forced, false))
	}
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := ts.MakeCtx(gh, ts.Ctx{At: onThe2nd.Add(12 * time.Hour), Cfg: cfg})
	planned := flowMake(ctx)
	assertNotEqual(t, planned.Get("provider"), ts.Weak)
	switchedOff := func() []int {
		opened := []int{}
		for n, thread := range gh.Threads.All() {
			if strings.Contains(providersStr(thread.Get("title")), "`"+ts.Weak+"` is switched off") {
				opened = append(opened, n)
			}
		}
		return opened
	}
	opened := switchedOff()
	assertEqual(t, len(opened), 1)
	body := providersStr(gh.Threads.Item(opened[0]).Get("body"))
	assertIn(t, "off_from: 2026-11-02", body)
	assertIn(t, "Its plan runs out", body)
	assertEqual(t, gh.LabelNames(opened[0]), setOf("night bot"))
	flowMake(ctx) // once only
	assertEqual(t, len(switchedOff()), 1)
	raw := ts.WithWeak(providersRaw())
	providersOf(raw, ts.Weak).Set("off_from", "mid-October")
	assertRaises[*herr.ConfigError](t, func() { providers.Parse(raw) })
}

// claude-1's window is 21:00–07:00 under a 100% cap on its 5-hour session with no weekly cap;
// outside it, it still works while its 5-hour usage is under 40% (`off_hours`), and a run there
// stops past 40%.
func TestProviders_Claude1WorksOutsideItsHoursUpTo40Percent(t *testing.T) {
	claude1 := providers.Load(ts.Root).Get("claude-1")
	assertEqual(t, claude1.Hours(providersZone),
		"21:00–07:00 America/Chicago, outside them up to 40% of 5-hour")
	later := clock.ISO(ts.Day.Add(12 * time.Hour)) // past both checks below
	everyone := providersEveryone()

	why := func(at time.Time, used float64) string {
		entry := pyjson.New("usage", pyjson.New("five_hour", providersReading(used, later)))
		return providers.Availability(claude1, providersState("claude-1", entry), at, providersZone,
			everyone, false, false)
	}

	night := ts.Day.Add(10 * time.Hour) // 22:00 CDT, inside the window
	assertIsNone(t, why(ts.Day, 0.3))
	assertIn(t, "5-hour usage is 45%, at or over its 40% cap outside its hours", why(ts.Day, 0.45))
	assertIsNone(t, why(night, 0.45))
	assertIsNone(t, why(night, 0.99)) // it may spend its whole session in its hours
	assertIn(t, "at or over its 100% cap; ", why(night, 1.0))
	// The mid-run stop holds the same caps.
	entry := pyjson.New("usage", pyjson.New("five_hour", providersReading(0.41, later)))
	assertNotEqual(t, providers.Refusal(claude1, entry, ts.Day, providersZone, false, time.Time{}), "")
	assertIsNone(t, providers.Refusal(claude1, entry, night, providersZone, false, time.Time{}))
	raw := providersRaw()
	providersOf(raw, "claude-1").Set("off_hours", pyjson.New("five_hours", 0.4))
	assertRaises[*herr.ConfigError](t, func() { providers.Parse(raw) })
}

// providersTwoWindows is {"usage": {"five_hour": ..., "seven_day": ...}}, both resetting `later`.
func providersTwoWindows(fiveHour, sevenDay float64, later string) *pyjson.Object {
	return pyjson.New("usage", pyjson.New("five_hour", providersReading(fiveHour, later),
		"seven_day", providersReading(sevenDay, later)))
}

// claude-4's window is 03:00–15:00 Central under a 70% cap on its 5-hour session with no weekly
// cap; outside it, it still works while that session is under 50% (`off_hours`).
func TestProviders_Claude4WorksAnyHourUnderHalfAndUpTo70PercentFrom3To15(t *testing.T) {
	claude4 := providers.Load(ts.Root).Get("claude-4")
	assertEqual(t, []any{claude4.Secret, claude4.Schedule.Mode, claude4.Limits.Stops(), claude4.OffHours},
		[]any{"CLAUDE_CODE_OAUTH_TOKEN_4", "window", pyjson.New("five_hour", 0.7), pyjson.New("five_hour", 0.5)})
	assertEqual(t, claude4.Hours(providersZone),
		"03:00–15:00 America/Chicago, outside them up to 50% of 5-hour")
	later := clock.ISO(ts.Day.Add(24 * time.Hour)) // past every check below
	everyone := providersEveryone()

	why := func(at time.Time, fiveHour, sevenDay float64, starting bool) string {
		return providers.Availability(claude4, providersState("claude-4",
			providersTwoWindows(fiveHour, sevenDay, later)), at, providersZone, everyone, false, starting)
	}

	morning := ts.Day.Add(-2 * time.Hour) // 10:00 CDT, inside the window
	evening := ts.Day.Add(5 * time.Hour)  // 17:00 CDT, outside it
	night := ts.Day.Add(10 * time.Hour)   // 22:00 CDT, outside it
	assertIsNone(t, why(morning, 0.6, 0.1, false))
	// No weekly cap: a nearly spent week holds nothing back, in or out of the window.
	assertIsNone(t, why(morning, 0.1, 0.99, false))
	assertIn(t, "5-hour usage is 72%, at or over its 70% cap; ", why(morning, 0.72, 0.1, false))
	for _, outside := range []time.Time{evening, night} {
		assertIsNone(t, why(outside, 0.3, 0.1, false))
		assertIsNone(t, why(outside, 0.3, 0.99, false))
		assertIn(t, "5-hour usage is 55%, at or over its 50% cap outside its hours",
			why(outside, 0.55, 0.1, false))
	}
	// A build starts only `start_headroom` under the cap that holds then.
	assertIsNone(t, why(morning, 0.5, 0.1, true))
	assertIn(t, "too close to its 70% cap", why(morning, 0.67, 0.1, true))
	assertIn(t, "too close to its 50% cap outside its hours", why(evening, 0.47, 0.1, true))
	// A run going on stops at the cap that holds when it is checked.
	entry := pyjson.New("usage", pyjson.New("five_hour", providersReading(0.55, later)))
	assertIsNone(t, providers.Refusal(claude4, entry, morning, providersZone, false, time.Time{}))
	assertNotEqual(t, providers.Refusal(claude4, entry, evening, providersZone, false, time.Time{}), "")
}

// claude-6 runs under the same rules as claude-4: the 03:00–15:00 window up to 70% of its 5-hour
// session, under 50% outside it, and no weekly cap.
func TestProviders_Claude6MatchesClaude4WithNoWeeklyCap(t *testing.T) {
	pool := providers.Load(ts.Root)
	claude6 := pool.Get("claude-6")
	claude4 := pool.Get("claude-4")
	assertEqual(t, claude6.Secret, "CLAUDE_CODE_OAUTH_TOKEN_6")
	assertEqual(t, []any{*claude6.Schedule, claude6.Limits.Stops(), claude6.OffHours},
		[]any{*claude4.Schedule, claude4.Limits.Stops(), claude4.OffHours})
	assertEqual(t, claude6.Hours(providersZone), claude4.Hours(providersZone))
	later := clock.ISO(ts.Day.Add(24 * time.Hour))
	everyone := providersEveryone()

	why := func(at time.Time, fiveHour, sevenDay float64) string {
		return providers.Availability(claude6, providersState("claude-6",
			providersTwoWindows(fiveHour, sevenDay, later)), at, providersZone, everyone, false, false)
	}

	morning := ts.Day.Add(-2 * time.Hour) // 10:00 CDT, inside the window
	evening := ts.Day.Add(5 * time.Hour)  // 17:00 CDT, outside it
	assertIsNone(t, why(morning, 0.6, 0.99))
	assertIn(t, "5-hour usage is 72%, at or over its 70% cap; ", why(morning, 0.72, 0.1))
	assertIsNone(t, why(evening, 0.3, 0.99))
	assertIn(t, "5-hour usage is 55%, at or over its 50% cap outside its hours", why(evening, 0.55, 0.1))
}

// claude-7's window is 01:00–10:00 Central under a 70% cap on its 5-hour session with no weekly
// cap; from 10:00 to 01:00 it still works while that session is under 50% (`off_hours`).
func TestProviders_Claude7WorksAnyHourUnderHalfAndUpTo70PercentFrom1To10(t *testing.T) {
	claude7 := providers.Load(ts.Root).Get("claude-7")
	assertEqual(t, []any{claude7.Secret, claude7.Schedule.Mode, claude7.Limits.Stops(), claude7.OffHours},
		[]any{"CLAUDE_CODE_OAUTH_TOKEN_7", "window", pyjson.New("five_hour", 0.7), pyjson.New("five_hour", 0.5)})
	assertEqual(t, claude7.Hours(providersZone),
		"01:00–10:00 America/Chicago, outside them up to 50% of 5-hour")
	later := clock.ISO(ts.Day.Add(24 * time.Hour)) // past every check below
	everyone := providersEveryone()

	why := func(at time.Time, fiveHour, sevenDay float64, starting bool) string {
		return providers.Availability(claude7, providersState("claude-7",
			providersTwoWindows(fiveHour, sevenDay, later)), at, providersZone, everyone, false, starting)
	}

	early := ts.Day.Add(-(10*time.Hour + 30*time.Minute))  // 01:30 CDT, inside the window
	morning := ts.Day.Add(-(2*time.Hour + 30*time.Minute)) // 09:30 CDT, inside it
	lateMorning := ts.Day.Add(-2 * time.Hour)              // 10:00 CDT, the window has closed
	midnight := ts.Day.Add(12*time.Hour + 30*time.Minute)  // 00:30 CDT, outside it
	for _, inside := range []time.Time{early, morning} {
		assertIsNone(t, why(inside, 0.6, 0.1, false))
		// No weekly cap: a nearly spent week holds nothing back, in or out of the window.
		assertIsNone(t, why(inside, 0.1, 0.99, false))
		assertIn(t, "5-hour usage is 72%, at or over its 70% cap; ", why(inside, 0.72, 0.1, false))
	}
	for _, outside := range []time.Time{lateMorning, ts.Day, midnight} {
		assertIsNone(t, why(outside, 0.3, 0.1, false))
		assertIsNone(t, why(outside, 0.3, 0.99, false))
		assertIn(t, "5-hour usage is 55%, at or over its 50% cap outside its hours",
			why(outside, 0.55, 0.1, false))
	}
	// A build starts only `start_headroom` under the cap that holds then.
	assertIsNone(t, why(morning, 0.5, 0.1, true))
	assertIn(t, "too close to its 70% cap", why(morning, 0.67, 0.1, true))
	assertIn(t, "too close to its 50% cap outside its hours", why(ts.Day, 0.47, 0.1, true))
	// A run going on stops at the cap that holds when it is checked.
	entry := pyjson.New("usage", pyjson.New("five_hour", providersReading(0.55, later)))
	assertIsNone(t, providers.Refusal(claude7, entry, morning, providersZone, false, time.Time{}))
	assertNotEqual(t, providers.Refusal(claude7, entry, ts.Day, providersZone, false, time.Time{}), "")
	// Without its secret it takes nothing.
	assertIn(t, "CLAUDE_CODE_OAUTH_TOKEN_7", providers.Availability(claude7, pyjson.New(), ts.Night,
		providersZone, providers.SecretsOf(providersSecrets("CLAUDE_CODE_OAUTH_TOKEN")), false, false))
}

func TestProviders_Claude8WorksAnyHourUpTo85PercentOfItsSessionWithNoWeeklyCap(t *testing.T) {
	claude8 := providers.Load(ts.Root).Get("claude-8")
	assertEqual(t, []any{claude8.Secret, claude8.Schedule.Mode, claude8.Limits.Stops(), claude8.OffHours},
		[]any{"CLAUDE_CODE_OAUTH_TOKEN_8", "always", pyjson.New("five_hour", 0.85), pyjson.New()})
	assertEqual(t, claude8.Hours(providersZone), "any time")
	later := clock.ISO(ts.Day.Add(24 * time.Hour)) // past every check below
	everyone := providersEveryone()

	why := func(fiveHour, sevenDay float64, starting bool, at time.Time) string {
		return providers.Availability(claude8, providersState("claude-8",
			providersTwoWindows(fiveHour, sevenDay, later)), at, providersZone, everyone, false, starting)
	}

	for _, at := range []time.Time{ts.Day, ts.Night, ts.Day.Add(12*time.Hour + 30*time.Minute)} { // any hour
		assertIsNone(t, why(0.8, 0.1, false, at))
		// No weekly cap: a nearly spent week holds nothing back.
		assertIsNone(t, why(0.1, 0.99, false, at))
		assertIn(t, "5-hour usage is 86%, at or over its 85% cap", why(0.86, 0.1, false, at))
	}
	// A build starts only `start_headroom` (5 points) under the cap.
	assertIsNone(t, why(0.79, 0.1, true, ts.Day))
	assertIn(t, "too close to its 85% cap", why(0.82, 0.1, true, ts.Day))
	// A run going on stops at the cap.
	entry := pyjson.New("usage", pyjson.New("five_hour", providersReading(0.86, later)))
	assertNotEqual(t, providers.Refusal(claude8, entry, ts.Day, providersZone, false, time.Time{}), "")
	// Without its secret it takes nothing.
	assertIn(t, "CLAUDE_CODE_OAUTH_TOKEN_8", providers.Availability(claude8, pyjson.New(), ts.Night,
		providersZone, providers.SecretsOf(providersSecrets("CLAUDE_CODE_OAUTH_TOKEN")), false, false))
}

func TestProviders_Claude5WorksAnyHourUnder40PercentOfItsSessionAnd60OfItsWeek(t *testing.T) {
	claude5 := providers.Load(ts.Root).Get("claude-5")
	assertEqual(t, []any{claude5.Secret, claude5.Schedule.Mode, claude5.Limits.Stops()},
		[]any{"CLAUDE_CODE_OAUTH_TOKEN_5", "always", pyjson.New("five_hour", 0.4, "seven_day", 0.6)})
	later := clock.ISO(ts.Day.Add(24 * time.Hour))
	everyone := providersEveryone()

	why := func(at time.Time, fiveHour, sevenDay float64, starting bool) string {
		return providers.Availability(claude5, providersState("claude-5",
			providersTwoWindows(fiveHour, sevenDay, later)), at, providersZone, everyone, false, starting)
	}

	for _, at := range []time.Time{ts.Day, ts.Night} { // the same caps by day and by night
		assertIsNone(t, why(at, 0.35, 0.1, false))
		assertIn(t, "5-hour usage is 41%, at or over its 40% cap; ", why(at, 0.41, 0.1, false))
		assertIsNone(t, why(at, 0.1, 0.55, false))
		assertIn(t, "7-day usage is 61%, at or over its 60% cap; ", why(at, 0.1, 0.61, false))
	}
	// A build starts only `start_headroom` under the caps: under 35% and 55%.
	assertIsNone(t, why(ts.Day, 0.3, 0.1, true))
	assertIn(t, "too close to its 40% cap", why(ts.Day, 0.37, 0.1, true))
	assertIn(t, "too close to its 60% cap", why(ts.Day, 0.1, 0.57, true))
}

func TestProviders_TheMediumModelsMaySpendTheirWholeWeek(t *testing.T) {
	pool := providers.Load(ts.Root)
	got := pyjson.New()
	for _, pid := range []string{"gpt", "agy", "muse"} {
		got.Set(pid, pool.Get(pid).Limits.Stops().Get("seven_day"))
	}
	assertEqual(t, got, pyjson.New("gpt", 1.0, "agy", 1.0, "muse", 1.0))
}

// #682: a subscription's own `start_headroom` takes the place of the pool's, window by window.
// Muse's week and Squishy's have none, so a build or a plan starts right up to the weekly cap,
// 100% for both; their 5-hour windows, and every other subscription's two, keep the pool's 5
// points.
func TestProviders_MuseAndSquishyMayStartRightUpToTheirWeeklyCaps(t *testing.T) {
	pool := providers.Load(ts.Root)
	assertEqual(t, pool.Get("muse").Limits.Headroom, pyjson.New("five_hour", 0.05, "seven_day", 0.0))
	for _, pid := range []string{"agy", "gpt", "claude-2", "claude-5"} {
		assertEqual(t, pool.Get(pid).Limits.Headroom, pyjson.New("five_hour", 0.05, "seven_day", 0.05), pid)
	}
	squishy := providers.Parse(providersLoad(filepath.Join(ts.Root, ".squishy", "providers.json"))).
		Get("claude-squishy")
	assertEqual(t, squishy.Limits.Stops(), pyjson.New("five_hour", 0.95, "seven_day", 1.0))
	assertEqual(t, providers.StartUnder(squishy, "seven_day", 1.0), 1.0)
	assertAlmostEqual(t, providers.StartUnder(squishy, "five_hour", 0.95), 0.9)

	// The tests' day comes before Muse's `usage_since`, which would set its reading aside.
	muse := *pool.Get("muse")
	muse.UsageSince = time.Time{}
	later := clock.ISO(ts.Day.Add(24 * time.Hour))

	why := func(sevenDay float64, starting bool) string {
		entry := pyjson.New("usage", pyjson.New("observed_at", clock.ISO(ts.Day),
			"five_hour", providersReading(0.1, later), "seven_day", providersReading(sevenDay, later)))
		return providers.Refusal(&muse, entry, ts.Day, providersZone, starting, time.Time{})
	}

	assertIsNone(t, why(0.99, true))
	assertIn(t, "7-day usage is 100%", why(1.0, true))
	assertIn(t, "7-day usage is 100%", why(1.0, false))
}

func TestProviders_ASubscriptionsStartHeadroomIsChecked(t *testing.T) {
	for _, c := range []struct {
		bad     *pyjson.Object
		message string
	}{
		{pyjson.New("day", 0.1), `unknown windows day`},
		{pyjson.New("seven_day", 1.0), `outside \[0, 1\)`},
		{pyjson.New("seven_day", -0.1), `outside \[0, 1\)`},
	} {
		raw := providersRaw()
		providersOf(raw, "agy").Set("start_headroom", c.bad)
		assertRaisesRegex[*herr.ConfigError](t, c.message, func() { providers.Parse(raw) })
	}
}

// The committed hours: claude-1, claude-2 and claude-3 run all day, claude-1 under its 100% cap
// with no weekly cap, claude-2 under 90% and claude-3 with none; claude-4 and claude-6 run all day
// too, up to 70% of their 5-hour session from 03:00 to 15:00 and under 50% otherwise, claude-7 the
// same from 01:00 to 10:00, all three with no weekly cap, claude-5 under 40%/60% and claude-8
// under 85% of its 5-hour session at any hour, with no weekly cap (their own tests are above). By
// day claude-3, claude-7, claude-8, claude-1, claude-4, claude-6 and then claude-5 take Opus's
// work first, and claude-2, kept back, plans for the medium models.
func TestProviders_Claude2And3WorkAnyHour(t *testing.T) {
	pool := providers.Load(ts.Root)
	hours := map[string][]any{}
	for _, p := range pool.Ordered() {
		if p.CLI == "claude" {
			hours[p.ID] = []any{p.Schedule.Mode, p.Limits.Mode}
		}
	}
	assertEqual(t, hours, map[string][]any{
		"claude-1": {"window", "caps"}, "claude-2": {"always", "caps"},
		"claude-3": {"always", "none"}, "claude-4": {"window", "caps"},
		"claude-5": {"always", "caps"},
		"claude-6": {"window", "caps"},
		"claude-7": {"window", "caps"},
		"claude-8": {"always", "caps"}})
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	ctx := providersCtxFor(gh, providersCtx{At: ts.Day, CommittedHours: true})
	// Room for all eight Claude lanes: the order they fill in is this test's, the cap on GitHub's
	// runners (`max_parallel`, #1455) its own.
	ctx = ts.MakeCtx(gh, ts.Ctx{At: ts.Day, Cfg: providersWithPool(ctx.Cfg, func(p *providers.Pool) {
		p.MaxParallel = 20
	})})
	planned := flowMake(ctx)
	assertEqual(t, providersPicked(planned), []any{4, "claude-3", "build"})
	providersRun(gh, "1", "in_progress")
	flowUpdateItem(ctx, 4, pyjson.New("run_id", "1"))
	planned = flowMake(ctx)
	// The unrated one goes to the weakest seat that meets its floor (#695): Muse plans it first in
	// its own build run.
	assertEqual(t, providersPicked(planned), []any{3, "muse", "build"})
	providersRun(gh, "2", "in_progress")
	flowUpdateItem(ctx, 3, pyjson.New("run_id", "2"))
	// The lanes fill in tier order (#695): Muse holds #3 on one of its four lanes (#1273), so it
	// takes the next three too, then agy, then Codex, then the Sonnet seats in usage order.
	for _, c := range []struct {
		number int
		run    string
	}{{6, "3"}, {15, "15"}, {16, "16"}} {
		gh.AddIssue(c.number, fakes.Issue{Labels: []string{config.LabelBuild}})
		planned = flowMake(ctx)
		assertEqual(t, providersPicked(planned), []any{c.number, "muse", "build"})
		providersRun(gh, c.run, "in_progress")
		flowUpdateItem(ctx, c.number, pyjson.New("run_id", c.run))
	}
	gh.AddIssue(5, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned = flowMake(ctx)
	// Muse's lanes are held: agy takes the next one.
	assertEqual(t, providersPicked(planned), []any{5, "agy", "build"})
	providersRun(gh, "4", "in_progress")
	flowUpdateItem(ctx, 5, pyjson.New("run_id", "4"))
	// Then Codex, after agy in the tier order and before Sonnet.
	gh.AddIssue(13, fakes.Issue{Labels: []string{config.LabelBuild}})
	planned = flowMake(ctx)
	assertEqual(t, providersPicked(planned), []any{13, "gpt", "build"})
	providersRun(gh, "13", "in_progress")
	flowUpdateItem(ctx, 13, pyjson.New("run_id", "13"))
	// Then the Sonnet seats: claude-3 holds #4 on one of its two lanes, so the load balancer
	// (`plan.balance`) gives each next one to an account holding none, in usage order (claude-1
	// works by day too, after claude-7 and claude-8), and claude-3's second lane comes after all
	// of them; the capped accounts hold one lane each (#695).
	for i, c := range []struct {
		number   int
		provider string
	}{{7, "claude-7"}, {8, "claude-8"}, {9, "claude-1"}, {10, "claude-4"}, {11, "claude-6"},
		{12, "claude-5"}, {14, "claude-3"}} {
		run := strconv.Itoa(i + 5)
		gh.AddIssue(c.number, fakes.Issue{Labels: []string{config.LabelBuild}})
		planned = flowMake(ctx)
		assertEqual(t, providersPicked(planned), []any{c.number, c.provider, "build"})
		providersRun(gh, run, "in_progress")
		flowUpdateItem(ctx, c.number, pyjson.New("run_id", run))
	}
	// Past 90% claude-2 is held; claude-3's readings never stop it, only a refusal does.
	later := clock.ISO(ts.Day.Add(2 * 24 * time.Hour))
	full := func() *pyjson.Object {
		return pyjson.New("five_hour", providersReading(0.99, later), "seven_day", providersReading(0.99, later))
	}
	st := pyjson.New("providers", pyjson.New("claude-2", pyjson.New("usage", full()),
		"claude-3", pyjson.New("usage", full())))
	everyone := providersEveryone()
	assertIn(t, "usage is 99%", providers.Availability(pool.Get("claude-2"), st, ts.Day, providersZone,
		everyone, false, false))
	assertIsNone(t, providers.Availability(pool.Get("claude-3"), st, ts.Day, providersZone, everyone,
		false, false))
}

// `balance` (on in providers.json): among the seats of one model, the subscription with the most
// room under its caps (`providers.room`) builds, whatever the usage order; level ones (the same
// `BALANCE_STEP`) go by the runs they hold, then the usage order. Off, the usage order alone
// decides, and claude-3 is spent first up to its cap.
func TestProviders_TheLoadBalancerSpreadsATierByRoom(t *testing.T) {
	later := clock.ISO(ts.Night.Add(2 * 24 * time.Hour))

	builder := func(claude3, claude7 float64, balance bool) any {
		gh := fakes.NewFakeGitHub()
		gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
		cfg := ts.MakeConfig(ts.Config{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_3",
			"CLAUDE_CODE_OAUTH_TOKEN_7")})
		ctx := ts.MakeCtx(gh, ts.Ctx{Cfg: providersWithPool(cfg, func(p *providers.Pool) { p.Balance = balance })})
		ctx.Store.Update(func(s *pyjson.Object) any {
			listed := pyjson.Obj(s.SetDefault("providers", pyjson.New()))
			for _, c := range []struct {
				name string
				week float64
			}{{"claude-3", claude3}, {"claude-7", claude7}} {
				listed.Set(c.name, pyjson.New("usage", pyjson.New("seven_day", providersReading(c.week, later))))
			}
			return nil
		}, "state")
		return flowMake(ctx).Get("provider")
	}

	assertEqual(t, builder(0.8, 0.1, true), "claude-7") // 20% of its week left to 90%
	assertEqual(t, builder(0.8, 0.1, false), "claude-3")
	assertEqual(t, builder(0.12, 0.08, true), "claude-3") // level: the usage order
	assertEqual(t, builder(0.05, 0.5, true), "claude-3")
	// Reviewers and planners go in the same order (`ranked`).
	pool := ts.MakeConfig().Pool
	seats := []*providers.Seat{}
	for _, name := range []string{"claude-7", "claude-3", "claude-1"} {
		seats = append(seats, pool.BestSeat(pool.Get(name), "weak"))
	}
	ids := func(seats []*providers.Seat) []string {
		out := []string{}
		for _, s := range seats {
			out = append(out, s.Provider.ID)
		}
		return out
	}
	assertEqual(t, ids(plan.Ranked(pool, seats, nil)), []string{"claude-3", "claude-7", "claude-1"})
	spread := plan.Balance{"claude-3": {-2, 1}, "claude-7": {-9, 0}, "claude-1": {-9, 1}}
	assertEqual(t, ids(plan.Ranked(pool, seats, spread)), []string{"claude-7", "claude-1", "claude-3"})
}

func TestProviders_HardWorkWaitsForAStrongModelAndComesFirst(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	planned := flowMake(providersCtxFor(gh))
	assertEqual(t, []any{planned.Get("number"), planned.Get("provider")}, []any{4, "claude-3"})
	gh = fakes.NewFakeGitHub()
	gh.AddIssue(4, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	planned = flowMake(providersCtxFor(gh, providersCtx{At: ts.Day}))
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, "outside its hours", planned.Get("reason"))
	assertEqual(t, providersForce(providersCtxFor(gh, providersCtx{At: ts.Day})).Get("provider"), "claude-3")
}

func TestProviders_EachSubscriptionTakesAsManyItemsAsItsLanes(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for _, n := range []int{3, 4, 5, 6, 7, 8, 9, 10, 11} {
		gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild}})
	}
	ctx := providersCtxFor(gh, providersCtx{At: ts.Day})
	taken := []any{}
	var planned *pyjson.Object
	for range 9 {
		planned = flowMake(ctx)
		if pyjson.Equal(planned.Get("action"), "none") {
			break
		}
		run := strconv.Itoa(len(taken))
		providersRun(gh, run, "in_progress")
		flowUpdateItem(ctx, planned.Int("number"), pyjson.New("run_id", run))
		taken = append(taken, []any{planned.Get("number"), planned.Get("provider")})
	}
	// Muse, first of the medium models, takes four items on its four lanes (#1273) before agy,
	// next in the tier order, and then gpt.
	assertEqual(t, taken, []any{[]any{3, "muse"}, []any{4, "muse"}, []any{5, "muse"}, []any{6, "muse"},
		[]any{7, "agy"}, []any{8, "gpt"}})
	// By day the Claude accounts are closed, and each machine subscription holds its lane.
	assertIn(t, "`muse` is busy", planned.Get("reason"))
	// Each claim with work and a lane left started the next run: Muse's four and agy's.
	assertEqual(t, len(gh.Dispatches), 5)
}

// Each machine subscription holds as many runs as its lanes (#695; Muse's are two, #1273); a
// Claude account on GitHub's runners still takes work, up to `max_parallel` in all.
func TestProviders_WithTheMachineSubscriptionsFullGithubTakesTheClaudeAccounts(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	for _, n := range []int{3, 4, 5, 6} {
		gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild}})
	}
	ctx := providersCtxFor(gh, providersCtx{At: ts.Night, Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN_2"),
		Machine: ts.Machine})
	for _, c := range []struct {
		n        int
		provider string
	}{{40, "gpt"}, {41, "agy"}, {42, "muse"}, {43, "muse"}, {44, "muse"}, {45, "muse"}} {
		gh.AddIssue(c.n, fakes.Issue{Labels: []string{config.LabelWorking}})
		providersRun(gh, strconv.Itoa(c.n), "in_progress")
		flowUpdateItem(ctx, c.n, pyjson.New("run_id", strconv.Itoa(c.n), "provider", c.provider))
	}
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("provider"), planned.Get("runs_on")}, []any{"claude-2", "ubuntu-latest"})
	// With one lane (#695) the Claude account takes nothing more while it holds its run: the next
	// item waits.
	providersRun(gh, "1", "in_progress")
	flowUpdateItem(ctx, planned.Int("number"), pyjson.New("run_id", "1"))
	again := flowMake(ctx)
	assertEqual(t, again.Get("action"), "none")
	assertIn(t, "`claude-2` is busy", again.Get("reason"))
}

// #1273: `lanes: 4`, under its one login: one runner on the night box and three on the Muse box.
// With gpt and agy at work (so no lane of theirs to fill, `fill_in`), a fifth item waits on its
// lanes, and a planning run counts in them, as on any capped subscription (`plan.Lanes.full`).
func TestProviders_MuseWorksFourItemsAtOnce(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	items := []int{3, 4, 5, 6, 7}
	for _, n := range items {
		gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:medium"}})
	}
	ctx := providersCtxFor(gh, providersCtx{At: ts.Day, Env: providersSecrets(), Machine: ts.Machine})
	for _, c := range []struct {
		n        int
		provider string
	}{{40, "gpt"}, {41, "agy"}} {
		gh.AddIssue(c.n, fakes.Issue{Labels: []string{config.LabelWorking}})
		providersRun(gh, strconv.Itoa(c.n), "in_progress")
		flowUpdateItem(ctx, c.n, pyjson.New("run_id", strconv.Itoa(c.n), "provider", c.provider))
	}
	ctx.Store.Update(func(s *pyjson.Object) any {
		for _, n := range items {
			state.Item(s, n).Set("planned_at", clock.ISO(ts.Day))
		}
		return nil
	}, "state")
	muse := ctx.Cfg.Pool.Get("muse")
	assertEqual(t, muse.Lanes, 4)
	taken := []any{}
	var planned *pyjson.Object
	for range 5 {
		planned = flowMake(ctx)
		if pyjson.Equal(planned.Get("action"), "none") {
			break
		}
		n := planned.Int("number")
		providersRun(gh, strconv.Itoa(n), "in_progress")
		flowUpdateItem(ctx, n, pyjson.New("run_id", strconv.Itoa(n)))
		taken = append(taken, []any{planned.Get("number"), planned.Get("provider")})
	}
	want := []any{}
	for _, n := range items[:4] {
		want = append(want, []any{n, "muse"})
	}
	assertEqual(t, taken, want)
	assertIn(t, "`muse` is busy", planned.Get("reason"))
	three := &plan.Lanes{Limit: 11, MachineLimit: 6, Machine: setOf("muse")}
	for _, n := range []int{3, 4, 5} {
		three = three.WithRun(n, "muse", "build", false)
	}
	assertFalse(t, three.Full(muse))
	assertTrue(t, three.WithRun(6, "muse", "plan", false).Full(muse))
}

// `lanes: 6`: the weak one takes more items while it holds some (six runners carry its label); a
// seventh waits on its lanes. The night box's limit of six counts every subscription's runs, so
// with another one there the weak one gets fewer.
func TestProviders_ASixLaneSubscriptionCanFillTheMachine(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	items := []int{3, 4, 5, 6, 7, 8, 9}
	for _, n := range items {
		gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"}}) // the weak one's
	}
	planAll := func(ctx *hctx.Context) {
		ctx.Store.Update(func(s *pyjson.Object) any {
			for _, n := range items {
				state.Item(s, n).Set("planned_at", clock.ISO(ts.Day))
			}
			return nil
		}, "state")
	}
	ctx := providersCtxFor(gh, providersCtx{At: ts.Day, Env: providersSecrets(), Machine: []string{"lite"}})
	planAll(ctx)
	assertEqual(t, []any{ctx.Cfg.Pool.Get("lite").Lanes, ctx.Cfg.Pool.MachineParallel}, []any{6, 6})
	taken := []any{}
	var planned *pyjson.Object
	for range 7 {
		planned = flowMake(ctx)
		if pyjson.Equal(planned.Get("action"), "none") {
			break
		}
		n := planned.Int("number")
		providersRun(gh, strconv.Itoa(n), "in_progress")
		flowUpdateItem(ctx, n, pyjson.New("run_id", strconv.Itoa(n)))
		taken = append(taken, []any{planned.Get("number"), planned.Get("provider")})
	}
	want := []any{}
	for _, n := range items[:6] {
		want = append(want, []any{n, "lite"})
	}
	assertEqual(t, taken, want)
	assertIn(t, "`lite` is busy", planned.Get("reason"))
	lanes := plan.ReadLanes(ctx, ctx.Store.Load())
	assertEqual(t, []any{lanes.Count("lite"), lanes.OnMachine(ctx.Cfg.Pool)}, []any{6, 6})
	// With GPT holding a machine run, the weak one takes five and waits for room on the box.
	gh = fakes.NewFakeGitHub()
	for _, n := range items {
		gh.AddIssue(n, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:easy"}})
	}
	ctx = providersCtxFor(gh, providersCtx{At: ts.Day, Env: providersSecrets(), Machine: []string{"lite", "gpt"}})
	planAll(ctx)
	gh.AddIssue(40, fakes.Issue{Labels: []string{config.LabelWorking}})
	providersRun(gh, "40", "in_progress")
	flowUpdateItem(ctx, 40, pyjson.New("run_id", "40", "provider", "gpt"))
	lites := 0
	for range 7 {
		planned = flowMake(ctx)
		if pyjson.Equal(planned.Get("action"), "none") {
			break
		}
		n := planned.Int("number")
		providersRun(gh, strconv.Itoa(n), "in_progress")
		flowUpdateItem(ctx, n, pyjson.New("run_id", strconv.Itoa(n)))
		if pyjson.Equal(planned.Get("provider"), "lite") {
			lites++
		}
	}
	assertEqual(t, lites, 5)
	assertIn(t, "`lite` waits for room on the machine (6 at once)", planned.Get("reason"))
}

func TestProviders_AnEasyAuditGoesToAMediumModel(t *testing.T) {
	// No weak review runs go out (#725): an easy item's weak approval is answered by the weakest
	// seat that ships it, medium.
	gh := fakes.NewFakeGitHub()
	gh.AddPull(9, "bot/issue-5", fakes.Pull{Labels: []string{config.LabelPR, config.LabelCross,
		"difficulty:easy"}, SHA: "h1"})
	ctx := providersCtxFor(gh, providersCtx{At: ts.Day})
	flowUpdateItem(ctx, 9, pyjson.New("votes", pyjson.New("sha", "h1", "builder", "gpt",
		"approvals", []any{"gpt"})))
	planned := flowMake(ctx)
	assertEqual(t, []any{planned.Get("action"), planned.Get("provider"), planned.Get("head"),
		planned.Get("builder")}, []any{"review", "muse", "h1", "gpt"})
	assertEqual(t, gh.LabelNames(9), setOf(config.LabelPR, config.LabelWorking, "difficulty:easy"))
}

// claude-1 is spent like any other subscription now: no gate wait for quiet, so the gate's word
// changes nothing.
func TestProviders_TheSharedSubscriptionTakesWorkWithoutAWait(t *testing.T) {
	env := providersSecrets("CLAUDE_CODE_OAUTH_TOKEN")
	for _, quietOK := range []string{"", "claude-1"} {
		gh := fakes.NewFakeGitHub()
		gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
		quiet := quietOK
		planned := plan.Make(providersCtxFor(gh, providersCtx{Env: env, Machine: []string{}}), false, nil,
			"auto", &quiet)
		assertEqual(t, []any{planned.Get("action"), planned.Get("provider")}, []any{"build", "claude-1"})
	}
}

// ---------------------------------------------------------------------------- PeekTests

// No subscription waits for quiet, so peek never names one: not with every secret, not with only
// the shared subscription's, not by day.
func TestProviders_TheGateNeverWaitsForQuiet(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	look := plan.Peek_(providersCtxFor(gh), false, nil, "auto")
	// Unrated, so the lane's weakest seat that meets its floor takes it (#695): Muse.
	assertEqual(t, []any{look.Work, look.Provider, look.QuietProvider}, []any{true, "muse", ""})
	look = plan.Peek_(providersCtxFor(gh, providersCtx{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN"),
		Machine: []string{}}), false, nil, "auto")
	assertEqual(t, []any{look.Work, look.Provider, look.QuietProvider, look.QuietSecret, look.Fallback},
		[]any{true, "claude-1", "", "", false})
	look = plan.Peek_(providersCtxFor(gh, providersCtx{At: ts.Day}), false, nil, "auto")
	assertEqual(t, []any{look.Provider, look.QuietProvider}, []any{"muse", ""})
}

// Even a run sitting at the quiet step holds nothing back: with no subscription waiting, a second
// run goes ahead on the shared one too.
func TestProviders_RunsNeverWaitForQuiet(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	gh.Runs.Set("50", pyjson.New("id", 50, "status", "in_progress"))
	gh.Jobs["50"] = []*pyjson.Object{pyjson.New("steps", []any{pyjson.New("name", plan.QuietStep,
		"status", "in_progress")})}
	look := plan.Peek_(providersCtxFor(gh), false, nil, "auto")
	// Unrated, so the lane's weakest seat that meets its floor takes it (#695): Muse.
	assertEqual(t, []any{look.Work, look.Provider, look.QuietProvider}, []any{true, "muse", ""})
	look = plan.Peek_(providersCtxFor(gh, providersCtx{Env: providersSecrets("CLAUDE_CODE_OAUTH_TOKEN"),
		Machine: []string{}}), false, nil, "auto")
	assertEqual(t, []any{look.Work, look.Provider, look.QuietProvider}, []any{true, "claude-1", ""})
}

// ---------------------------------------------------------------------------- StartTests

// providersEarly: 2026-10-09 06:00 UTC is 01:00 CDT: outside claude-4's and claude-6's
// 03:00–15:00 window.
var providersEarly = time.Date(2026, 10, 9, 6, 0, 0, 0, time.UTC)

// providersTooCloseToStart is too_close_to_start(): the night claude-4 and claude-6 sat idle with
// work queued: outside their hours, under their 50% cap there, but not 5 points under it
// (`start_headroom`); every other subscription refused until later.
func providersTooCloseToStart() *pyjson.Object {
	later := clock.ISO(providersEarly.Add(6 * time.Hour))
	listed := pyjson.New()
	for _, pid := range providersRaw().Obj("providers").Keys() {
		listed.Set(pid, pyjson.New("refused_until", later))
	}
	st := pyjson.New("providers", listed)
	listed.Set("claude-4", pyjson.New("usage", pyjson.New(
		"five_hour", providersReading(0.49, "2026-10-09T09:10:00Z"),
		"observed_at", "2026-10-09T04:37:07Z")))
	listed.Set("claude-6", pyjson.New("usage", pyjson.New(
		"five_hour", providersReading(0.45, "2026-10-09T07:50:00Z"),
		"observed_at", "2026-10-09T03:34:44Z")))
	return st
}

// A subscription under its cap but too close to it to start a build, a revision or a plan says
// so, says it still takes reviews, and says when it can start one again.
func TestProviders_TooCloseToStartSaysReviewsOnlyAndWhenItCan(t *testing.T) {
	pool := providers.Load(ts.Root)
	st := providersTooCloseToStart()
	everyone := providersEveryone()
	for _, c := range []struct{ pid, frees, wait string }{
		{"claude-4", "2026-10-09T08:00:00Z", "2h 00m"},
		{"claude-6", "2026-10-09T06:50:00Z", "50m"},
	} {
		provider := pool.Get(c.pid)
		// A review goes up to the cap, so it may start one.
		assertIsNone(t, providers.Availability(provider, st, providersEarly, providersZone, everyone,
			false, false))
		why := providers.StartReason(provider, st, providersEarly, providersZone, everyone)
		assertIn(t, "too close to its 50% cap outside its hours to start a long run "+
			"(it starts under 45%)", why)
		assertIn(t, "; it takes reviews only; it can start a build or a plan again in "+c.wait, why)
		entry := providers.PeekRecord(st, c.pid)
		// claude-4 frees when its window opens (70% cap, starts under 65%), before its reset;
		// claude-6 when its session's last hour lifts its cap (`last_hour`), an hour before its
		// reset and before its window opens.
		assertEqual(t, clock.ISO(providers.FreesAt(provider, entry, providersEarly, providersZone)), c.frees)
	}
	// Something no reset lifts gets no time.
	held := providersState("claude-4", pyjson.New("infra", pyjson.New("at", clock.ISO(providersEarly),
		"reason", "401")))
	why := providers.StartReason(pool.Get("claude-4"), held, providersEarly, providersZone, everyone)
	assertIn(t, "could not work", why)
	assertNotIn(t, "again in", why)
}

func TestProviders_AQueuedItemAndThePlannerSayTheSame(t *testing.T) {
	st := providersTooCloseToStart()
	everyone := providersEveryone()
	text := providers.WhenFree(ts.TestPool(ts.Config{CommittedHours: true}), st, providersEarly,
		providersZone, everyone)
	assertEqual(t, text, "when `claude-6` has room under its caps again (in 50m)")
	gh := fakes.NewFakeGitHub()
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild}})
	ctx := providersCtxFor(gh, providersCtx{At: providersEarly, Machine: []string{}, CommittedHours: true})
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Obj("providers").Update(st.Obj("providers"))
		return nil
	}, "seed")
	planned := flowMake(ctx)
	assertEqual(t, planned.Get("action"), "none")
	assertIn(t, "`claude-4` 5-hour usage is 49%, too close to its 50% cap", planned.Get("reason"))
	assertIn(t, "it can start a build or a plan again in 2h 00m", planned.Get("reason"))
	assertNotIn(t, "is free", planned.Get("reason"))
}

// ---------------------------------------------------------------------------- WhenTests

func TestProviders_WhenAQueuedItemRuns(t *testing.T) {
	onlyClaude := providers.SecretsOf(providersSecrets("CLAUDE_CODE_OAUTH_TOKEN"))
	text := providers.WhenFree(ts.TestPool(), pyjson.New(), ts.Day, providersZone, onlyClaude)
	assertIn(t, "when `claude-1` opens (21:00–07:00", text)
	text = providers.WhenFree(providers.Load(ts.Root), pyjson.New(), ts.Day, providersZone, onlyClaude)
	assertIn(t, "`muse`, `agy`, `gpt` can take it now)", text)
}

// ---------------------------------------------------------------------------- SharedPoolTests
//
// #1455: the bots share one pool of GitHub's runners, first come first served. Each counts the
// other's live runs against its `max_parallel` (`plan.others_on_github`).

func TestProviders_SquishysLiveRunsHoldSlotsTheNightBotWaitsFor(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := providersCtxFor(gh, providersCtx{At: ts.Night})
	squishy := ".github/workflows/squishy-run.yml"
	gh.Runs.Set("900", pyjson.New("status", "in_progress", "path", squishy))
	gh.Runs.Set("901", pyjson.New("status", "queued", "path", squishy))
	gh.Runs.Set("902", pyjson.New("status", "completed", "path", squishy))
	gh.Runs.Set("903", pyjson.New("status", "in_progress", "path", ".github/workflows/ci.yml"))
	lanes := plan.ReadLanes(ctx, ctx.Store.Load())
	assertEqual(t, lanes.Shared, 2)
	assertEqual(t, lanes.Free(), ctx.Cfg.Pool.MaxParallel-2)
	// Every slot Squishy's: a Claude account waits, a machine subscription does not.
	for n := 904; n < 904+ctx.Cfg.Pool.MaxParallel; n++ {
		gh.Runs.Set(strconv.Itoa(n), pyjson.New("status", "in_progress", "path", squishy))
	}
	gh.AddIssue(3, fakes.Issue{Labels: []string{config.LabelBuild, "difficulty:hard"}})
	assertEqual(t, flowMake(ctx).Get("action"), "none")
}

func TestProviders_TheNightBotsRunsOnTheMachineHoldNone(t *testing.T) {
	night := ".github/workflows/bot-night.yml"
	gh := fakes.NewFakeGitHub()
	ctx := providersCtxFor(gh, providersCtx{At: ts.Night})
	gh.Runs.Set("910", pyjson.New("status", "in_progress", "path", night,
		"display_title", "bot-night 5-1 on night-vm-muse"))
	gh.Runs.Set("911", pyjson.New("status", "in_progress", "path", night,
		"display_title", "bot-night 6-2 on ubuntu-latest"))
	gh.Runs.Set("912", pyjson.New("status", "queued", "path", night, "display_title", "bot-night"))
	other := &identity.Other{Name: "the night bot", Login: "jgoetzmann-bot", LabelPrefix: "bot:",
		Workflow: "bot-night.yml"}
	saved := plan.Others
	plan.Others = []*identity.Other{other}
	t.Cleanup(func() { plan.Others = saved })
	assertEqual(t, plan.OthersOnGithub(ctx), 2)
}

func TestProviders_AnUnreadableListCountsNone(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	ctx := providersCtxFor(gh, providersCtx{At: ts.Night})
	gh.Patch.ListRuns = func(workflow string, status string, limit int, cached bool) []*pyjson.Object {
		herr.Raise(&herr.GitHubError{Msg: "down", Status: 502})
		return nil
	}
	assertEqual(t, plan.OthersOnGithub(ctx), 0)
}

// ---------------------------------------------------------------------------- LastHourTests
//
// The last hour of a 5-hour session (`last_hour`): what is left of a session is lost at its
// reset, so in the hour before it a capped Claude account that nobody but the bot spent (its
// reading no further over its cap than the bot's own calls leave it) may run past its 5-hour cap,
// one run at a time, until the reset. Not claude-2, and claude-5 only in its late week
// (`LateWeekTests`).

// providersEvening: 2026-09-29 17:00 CDT, a Tuesday: outside claude-4's 03:00–15:00 window, where
// its cap is 50%.
var providersEvening = ts.Day.Add(5 * time.Hour)

// providersLastHourWhy is LastHourTests.why (at=EVENING, starting=True; sevenDay < 0 is
// seven_day=None).
func providersLastHourWhy(pool *providers.Pool, providerID string, used float64, resetsIn time.Duration,
	sevenDay float64) string {
	at := providersEvening
	usage := pyjson.New("five_hour", providersReading(used, clock.ISO(at.Add(resetsIn))))
	if sevenDay >= 0 {
		usage.Set("seven_day", providersReading(sevenDay, clock.ISO(at.Add(3*24*time.Hour))))
	}
	return providers.Availability(pool.Get(providerID), providersState(providerID, pyjson.New("usage", usage)),
		at, providersZone, providersEveryone(), false, true)
}

func TestProviders_EveryCappedClaudeAccountHasOne(t *testing.T) {
	pool := providers.Load(ts.Root)
	lastHour, takes := map[string]bool{}, map[string]bool{}
	for _, p := range pool.Ordered() {
		if p.LastHour {
			lastHour[p.ID] = true
		}
		if p.TakesLastHours() {
			takes[p.ID] = true
		}
	}
	assertEqual(t, lastHour, setOf("claude-1", "claude-4", "claude-6", "claude-7", "claude-8"))
	assertEqual(t, takes, setOf("claude-1", "claude-4", "claude-5", "claude-6", "claude-7", "claude-8"))
	raw := providersRaw()
	providersOf(raw, "claude-3").Set("last_hour", true) // no caps to lift
	assertRaisesRegex[*herr.ConfigError](t, "it lifts a 5-hour cap, and claude-3 has none",
		func() { providers.Parse(raw) })
}

func TestProviders_InItsLastHourAnAccountNobodyElseSpentStartsAgain(t *testing.T) {
	pool := providers.Load(ts.Root)
	why := func(id string, used float64, resetsIn time.Duration, sevenDay ...float64) string {
		week := -1.0
		if len(sevenDay) > 0 {
			week = sevenDay[0]
		}
		return providersLastHourWhy(pool, id, used, resetsIn, week)
	}
	hour := time.Hour
	assertIn(t, "5-hour usage is 50%, too close to its 50% cap outside its hours",
		why("claude-4", 0.5, hour+time.Minute))
	assertIsNone(t, why("claude-4", 0.5, hour))
	assertIsNone(t, why("claude-4", 0.47, 20*time.Minute)) // too close before
	assertIsNone(t, why("claude-4", 0.51, 20*time.Minute)) // a call's overshoot
	// Further past the cap, somebody else spent the session: it stays capped.
	assertIn(t, "5-hour usage is 60%, too close to its 50% cap outside its hours",
		why("claude-4", 0.6, 20*time.Minute))
	// claude-8 caps its session at 85% any hour; its last hour lifts that too.
	assertIn(t, "5-hour usage is 85%, too close to its 85% cap", why("claude-8", 0.85, 2*hour))
	assertIsNone(t, why("claude-8", 0.85, 59*time.Minute))
	// claude-2 has none: its 90% holds to the end of the session.
	assertIn(t, "5-hour usage is 90%, too close to its 90% cap", why("claude-2", 0.9, 20*time.Minute))
	// Nor has claude-5 outside its late week (a Tuesday): its 40% holds.
	assertIn(t, "5-hour usage is 40%, too close to its 40% cap", why("claude-5", 0.4, 20*time.Minute, 0.3))
	// No reading, or one with no reset, never opens it.
	claude4 := pool.Get("claude-4")
	assertTrue(t, providers.LastHourOf(claude4, pyjson.New(), providersEvening, providersZone).IsZero())
	assertTrue(t, providers.LastHourOf(claude4, pyjson.New("five_hour", pyjson.New("utilization", 0.5,
		"resets_at", nil)), providersEvening, providersZone).IsZero())
}

func TestProviders_ARunInItsLastHourGoesOnPastTheCapUntilTheReset(t *testing.T) {
	pool := providers.Load(ts.Root)
	claude4 := pool.Get("claude-4")
	resets := providersEvening.Add(30 * time.Minute)
	start := pyjson.New("five_hour", providersReading(0.5, clock.ISO(resets)))
	assertEqual(t, providers.LastHourOf(claude4, start, providersEvening, providersZone), resets)
	assertAlmostEqual(t, providers.Room(claude4, pyjson.New("usage", start), providersEvening, providersZone), 0.5)
	later := providersEvening.Add(10 * time.Minute)

	reading := func(used float64, until time.Time) *pyjson.Object {
		return pyjson.New("usage", pyjson.New("five_hour", providersReading(used, clock.ISO(until))))
	}

	// Its own spending takes the reading past the cap; read alone, that looks like somebody
	// else's, so the run carries the session it took (`work.Worker.last_hour`).
	assertIn(t, "80%", providers.Refusal(claude4, reading(0.8, resets), later, providersZone, false, time.Time{}))
	assertIsNone(t, providers.Refusal(claude4, reading(0.8, resets), later, providersZone, false, resets))
	assertIn(t, "at or over its 100% cap", providers.Refusal(claude4, reading(1.0, resets), later,
		providersZone, false, resets))
	// After the reset the new session's 50% holds again.
	after := resets.Add(time.Minute)
	assertIn(t, "5-hour usage is 55%, at or over its 50% cap outside its hours",
		providers.Refusal(claude4, reading(0.55, resets.Add(5*time.Hour)), after, providersZone, false, resets))
	// At its cap with two hours left, it can start again in one, when its last hour opens.
	entry := reading(0.5, providersEvening.Add(2*time.Hour))
	assertEqual(t, providers.FreesAt(claude4, entry, providersEvening, providersZone),
		providersEvening.Add(time.Hour))
	ctx := ts.MakeCtx(nil, ts.Ctx{At: providersEvening, Cfg: ts.MakeConfig(ts.Config{CommittedHours: true})})
	// The work job's check between steps holds the run's session too (`make_probe`).
	spent := reading(0.8, resets).Obj("usage")
	reason, _, stopped := cli.MakeProbe(ctx, nil, claude4, nil)(spent)
	assertTrue(t, stopped)
	assertIn(t, "`claude-4` stops: 5-hour usage is 80%", reason)
	_, _, stopped = cli.MakeProbe(ctx, nil, claude4, func() time.Time { return resets })(spent)
	assertFalse(t, stopped)
	assertEqual(t, status.CapText(ctx, claude4, "five_hour", start),
		"cap 100% now (its last hour), 50% after the reset")
	assertEqual(t, status.CapText(ctx, claude4, "five_hour", nil), "cap 50% now, 70% in its hours")
}

// ---------------------------------------------------------------------------- LateWeekTests
//
// claude-5's late week (`late_week`): from Sunday 17:00 Central until its week resets (Monday
// 09:00 UTC), it may spend all of its week instead of 60%, 80% of a 5-hour session instead of 40%,
// and a session's last hour (`LastHourTests`), which it takes nowhere else.

// providersSunday: Sunday 2026-11-01, the day the clocks go back: 17:00 CST is 23:00 UTC, ten
// hours before its week resets at 03:00 CST.
var providersSunday = time.Date(2026, 11, 1, 23, 0, 0, 0, time.UTC)

// providersResets is LateWeekTests.RESETS.
const providersResets = "2026-11-02T09:00:00Z"

// providersLateReading is LateWeekTests.usage's keyword arguments; a zero field is its default.
type providersLateReading struct {
	Session       *float64      // = 0.1
	SessionResets time.Duration // = timedelta(hours=3)
	Resets        string        // = RESETS
	Starting      *bool         // why's starting = True
}

// providersLateUsage is LateWeekTests.usage.
func providersLateUsage(at time.Time, week float64, o providersLateReading) *pyjson.Object {
	session := 0.1
	if o.Session != nil {
		session = *o.Session
	}
	sessionResets := o.SessionResets
	if sessionResets == 0 {
		sessionResets = 3 * time.Hour
	}
	resets := o.Resets
	if resets == "" {
		resets = providersResets
	}
	return pyjson.New("five_hour", providersReading(session, clock.ISO(at.Add(sessionResets))),
		"seven_day", providersReading(week, resets))
}

// providersLateWhy is LateWeekTests.why.
func providersLateWhy(claude5 *providers.Provider, at time.Time, week float64, opts ...providersLateReading) string {
	var o providersLateReading
	if len(opts) > 0 {
		o = opts[0]
	}
	starting := true
	if o.Starting != nil {
		starting = *o.Starting
	}
	entry := pyjson.New("usage", providersLateUsage(at, week, o))
	return providers.Availability(claude5, providersState("claude-5", entry), at, providersZone,
		providersEveryone(), false, starting)
}

func TestProviders_FromSunday1700UntilTheResetItsCapsGoUp(t *testing.T) {
	claude5 := providers.Load(ts.Root).Get("claude-5")
	why := func(at time.Time, week float64, opts ...providersLateReading) string {
		return providersLateWhy(claude5, at, week, opts...)
	}
	late := claude5.LateWeek
	assertEqual(t, []any{late.Describe(), late.Caps(), late.LastHour},
		[]any{"Sun 17:00", pyjson.New("five_hour", 0.8, "seven_day", 1.0), true})
	assertEqual(t, claude5.Limits.Stops(), pyjson.New("five_hour", 0.4, "seven_day", 0.6))
	assertFalse(t, claude5.LastHour)
	sunday := providersSunday
	// The week: 60% until 17:00, then all of it (a build starts under 95%).
	assertIn(t, "7-day usage is 70%, too close to its 60% cap", why(sunday.Add(-time.Minute), 0.7))
	assertIsNone(t, why(sunday, 0.7))
	assertIsNone(t, why(sunday.Add(9*time.Hour+59*time.Minute), 0.94))
	assertIn(t, "7-day usage is 96%, too close to its 100% cap to start a long run (it "+
		"starts under 95%)", why(sunday, 0.96))
	// a review goes up to the cap
	assertIsNone(t, why(sunday, 0.96, providersLateReading{Starting: ptr(false)}))
	assertIn(t, "7-day usage is 100%, at or over its 100% cap",
		why(sunday, 1.0, providersLateReading{Starting: ptr(false)}))
	// The session: 40% until 17:00, then 80% (a build starts under 75%).
	assertIn(t, "5-hour usage is 50%, too close to its 40% cap",
		why(sunday.Add(-time.Minute), 0.3, providersLateReading{Session: ptr(0.5)}))
	assertIsNone(t, why(sunday, 0.3, providersLateReading{Session: ptr(0.5)}))
	assertIn(t, "5-hour usage is 78%, too close to its 80% cap",
		why(sunday, 0.3, providersLateReading{Session: ptr(0.78)}))
	assertIn(t, "5-hour usage is 81%, at or over its 80% cap",
		why(sunday, 0.3, providersLateReading{Session: ptr(0.81), Starting: ptr(false)}))
	// A session's last hour, at its 80% cap and nobody else's spending, lifts that cap; past it by
	// more than a point, somebody else spent it, and it holds.
	last := 30 * time.Minute
	assertIsNone(t, why(sunday, 0.3, providersLateReading{Session: ptr(0.8), SessionResets: last}))
	assertIn(t, "5-hour usage is 85%, too close to its 80% cap",
		why(sunday, 0.3, providersLateReading{Session: ptr(0.85), SessionResets: last}))
	assertEqual(t, providers.LastHourOf(claude5, providersLateUsage(sunday, 0.3,
		providersLateReading{Session: ptr(0.8), SessionResets: last}), sunday, providersZone), sunday.Add(last))
	// Its week's cap still holds in that last hour.
	assertIn(t, "7-day usage is 100%, too close to its 100% cap",
		why(sunday, 1.0, providersLateReading{Session: ptr(0.8), SessionResets: last}))
	// The new week holds to 60% and 40% again, with no last hour.
	monday := time.Date(2026, 11, 2, 9, 30, 0, 0, time.UTC)
	week := "2026-11-09T09:00:00Z"
	assertIn(t, "7-day usage is 65%, too close to its 60% cap",
		why(monday, 0.65, providersLateReading{Resets: week}))
	assertIn(t, "5-hour usage is 50%, too close to its 40% cap",
		why(monday, 0.1, providersLateReading{Session: ptr(0.5), Resets: week}))
	assertIn(t, "5-hour usage is 40%, too close to its 40% cap",
		why(monday, 0.1, providersLateReading{Session: ptr(0.4), SessionResets: last, Resets: week}))
	// A week whose reset moved days off (to Wednesday) raises nothing on Sunday.
	assertIn(t, "too close to its 60% cap", why(sunday.Add(time.Hour), 0.7,
		providersLateReading{Resets: "2026-11-04T09:00:00Z"}))
	// In summer time, 17:00 CDT is 22:00 UTC.
	week = "2026-10-26T09:00:00Z"
	assertIn(t, "7-day usage is 70%, too close to its 60% cap",
		why(time.Date(2026, 10, 25, 21, 59, 0, 0, time.UTC), 0.7, providersLateReading{Resets: week}))
	assertIsNone(t, why(time.Date(2026, 10, 25, 22, 0, 0, 0, time.UTC), 0.7, providersLateReading{Resets: week}))
}

func TestProviders_ItSaysWhenItStartsAndCountsItsRoom(t *testing.T) {
	claude5 := providers.Load(ts.Root).Get("claude-5")
	noon := time.Date(2026, 11, 1, 18, 0, 0, 0, time.UTC)
	entry := pyjson.New("usage", pyjson.New("seven_day", providersReading(0.62, providersResets)))
	assertEqual(t, providers.FreesAt(claude5, entry, noon, providersZone), providersSunday)
	assertEqual(t, providers.Room(claude5, entry, noon, providersZone), 0.0)
	assertAlmostEqual(t, providers.Room(claude5, entry, providersSunday, providersZone), 0.38)
	ctx := ts.MakeCtx(nil, ts.Ctx{At: providersSunday, Cfg: ts.MakeConfig(ts.Config{CommittedHours: true})})

	capText := func(window string, o providersLateReading) string {
		return status.CapText(ctx, claude5, window, providersLateUsage(providersSunday, 0.62, o))
	}

	assertEqual(t, capText("seven_day", providersLateReading{}),
		"cap 100% now (its late week, from Sun 17:00), 60% after the reset")
	assertEqual(t, capText("five_hour", providersLateReading{Session: ptr(0.5)}),
		"cap 80% now (its late week, from Sun 17:00), 40% after the reset")
	assertEqual(t, capText("five_hour", providersLateReading{Session: ptr(0.8), SessionResets: 30 * time.Minute}),
		"cap 100% now (its last hour), 80% after the reset")
}

func TestProviders_LateWeekTests_MistakesAreErrors(t *testing.T) {
	for _, c := range []struct {
		providerID string
		bad        *pyjson.Object
		message    string
	}{
		{"claude-5", pyjson.New("from", "Someday 17:00", "seven_day", 0.9), "not a weekday and a time"},
		{"claude-5", pyjson.New("from", "Sun 25:00", "seven_day", 0.9), "Sun 25:00"},
		{"claude-5", pyjson.New("from", "Sun 17:00"), "needs from"},
		{"claude-5", pyjson.New("from", "Sun 17:00", "seven_day", 0.9, "until", "Mon"), "unknown keys until"},
		{"claude-5", pyjson.New("from", "Sun 17:00", "seven_day", 0.5), "it raises a 7-day cap"},
		{"claude-5", pyjson.New("from", "Sun 17:00", "seven_day", 1.0, "five_hour", 0.3),
			"it raises a 5-hour cap, and claude-5's is not under 30%"},
		{"claude-4", pyjson.New("from", "Sun 17:00", "seven_day", 0.9), "claude-4's is not under"},
		{"claude-3", pyjson.New("from", "Sun 17:00", "seven_day", 0.9, "last_hour", true), "claude-3's is not under"},
	} {
		raw := providersRaw()
		providersOf(raw, c.providerID).Set("late_week", c.bad)
		assertRaisesRegex[*herr.ConfigError](t, c.message, func() { providers.Parse(raw) })
	}
	raw := providersRaw()
	providersOf(raw, "claude-5").Set("limits", pyjson.New("mode", "caps", "seven_day", 0.6))
	providersOf(raw, "claude-5").Set("late_week", pyjson.New("from", "Sun 17:00", "seven_day", 1.0,
		"last_hour", true))
	assertRaisesRegex[*herr.ConfigError](t, "late_week.last_hour: it lifts a 5-hour cap",
		func() { providers.Parse(raw) })
}
