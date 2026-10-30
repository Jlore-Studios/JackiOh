// basics_test.go is bot/tests/test_basics.py: config, clock and window, commands, trust,
// redaction, verdicts, prompts, state.
package tests

import (
	"encoding/base64"
	"fmt"
	"math"
	"os"
	"path/filepath"
	"reflect"
	"slices"
	"sort"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/commands"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	ghpkg "github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ------------------------------------------------------------------ unittest's assertions
//
// The assertions this file and its siblings of the same port (squishy, squishy_cases, journal,
// titles, triage, wake, quiet) use: unittest's, prefixed with this file's name so they never meet
// another file's in package tests.

// basicsMsg is the `msg` argument an assertion may be given.
func basicsMsg(msg []any) string {
	if len(msg) == 0 {
		return ""
	}
	return fmt.Sprint(msg...) + ": "
}

// basicsSame is Python's == as assertEqual reads it: two empty lists or dicts are equal however
// Go holds them, and JSON values compare as Python's (an int equals a float of the same value).
func basicsSame(got, want any) bool {
	if reflect.DeepEqual(got, want) {
		return true
	}
	if gt, ok := got.(time.Time); ok { // aware datetimes are equal at the same instant
		wt, ok := want.(time.Time)
		return ok && gt.Equal(wt)
	}
	g, w := reflect.ValueOf(got), reflect.ValueOf(want)
	if g.IsValid() && w.IsValid() && g.Type() == w.Type() {
		switch g.Kind() {
		case reflect.Slice, reflect.Map:
			if g.Len() == 0 && w.Len() == 0 {
				return true
			}
		}
	}
	return pyjson.Equal(got, want)
}

// basicsShow prints a value for a failure: JSON as JSON.
func basicsShow(v any) string {
	switch x := v.(type) {
	case *pyjson.Object, []any:
		return pyjson.Dumps(x)
	case string:
		return fmt.Sprintf("%q", x)
	}
	return fmt.Sprintf("%#v", v)
}

// basicsEqual is assertEqual.
func basicsEqual(t *testing.T, got, want any, msg ...any) {
	t.Helper()
	if !basicsSame(got, want) {
		t.Errorf("%snot equal\n got: %s\nwant: %s", basicsMsg(msg), basicsShow(got), basicsShow(want))
	}
}

// basicsNotEqual is assertNotEqual.
func basicsNotEqual(t *testing.T, got, other any, msg ...any) {
	t.Helper()
	if basicsSame(got, other) {
		t.Errorf("%sboth are %s", basicsMsg(msg), basicsShow(got))
	}
}

// basicsAlmostEqual is assertAlmostEqual: equal to seven places.
func basicsAlmostEqual(t *testing.T, got, want float64, msg ...any) {
	t.Helper()
	if got != want && py.RoundN(math.Abs(got-want), 7) != 0 {
		t.Errorf("%s%v != %v within 7 places", basicsMsg(msg), got, want)
	}
}

// basicsTrue is assertTrue.
func basicsTrue(t *testing.T, ok bool, msg ...any) {
	t.Helper()
	if !ok {
		t.Errorf("%sfalse is not true", basicsMsg(msg))
	}
}

// basicsFalse is assertFalse.
func basicsFalse(t *testing.T, ok bool, msg ...any) {
	t.Helper()
	if ok {
		t.Errorf("%strue is not false", basicsMsg(msg))
	}
}

// basicsHas is Python's `needle in haystack`: a substring, a member of a list, a key of a dict
// or a set.
func basicsHas(needle, haystack any) bool {
	switch h := haystack.(type) {
	case string:
		return strings.Contains(h, needle.(string))
	case []string:
		return slices.Contains(h, needle.(string))
	case map[string]bool:
		_, ok := h[needle.(string)]
		return ok
	case map[string]string:
		_, ok := h[needle.(string)]
		return ok
	case *pyjson.Object:
		return h.Has(needle.(string))
	case []any:
		for _, e := range h {
			if basicsSame(e, needle) {
				return true
			}
		}
		return false
	}
	v := reflect.ValueOf(haystack)
	switch v.Kind() {
	case reflect.Slice, reflect.Array:
		for i := range v.Len() {
			if basicsSame(v.Index(i).Interface(), needle) {
				return true
			}
		}
		return false
	case reflect.Map:
		return v.MapIndex(reflect.ValueOf(needle)).IsValid()
	}
	panic(fmt.Sprintf("basicsHas: cannot look in a %T", haystack))
}

// basicsIn is assertIn.
func basicsIn(t *testing.T, needle, haystack any, msg ...any) {
	t.Helper()
	if !basicsHas(needle, haystack) {
		t.Errorf("%s%s not found in %s", basicsMsg(msg), basicsShow(needle), basicsShow(haystack))
	}
}

// basicsNotIn is assertNotIn.
func basicsNotIn(t *testing.T, needle, haystack any, msg ...any) {
	t.Helper()
	if basicsHas(needle, haystack) {
		t.Errorf("%s%s unexpectedly found in %s", basicsMsg(msg), basicsShow(needle), basicsShow(haystack))
	}
}

// basicsRaises is assertRaises(E): fn must raise an E, which it returns.
func basicsRaises[E error](t *testing.T, fn func(), msg ...any) E {
	t.Helper()
	caught, ok := herr.Catch[E](fn)
	if !ok {
		var want E
		t.Errorf("%s%T not raised", basicsMsg(msg), want)
	}
	return caught
}

// basicsSet is a set of strings, as Python's {…}.
func basicsSet(names ...string) map[string]bool {
	out := map[string]bool{}
	for _, n := range names {
		out[n] = true
	}
	return out
}

// basicsSorted is sorted(names).
func basicsSorted(names []string) []string {
	out := slices.Clone(names)
	sort.Strings(out)
	return out
}

// basicsKeys is sorted(set_or_dict).
func basicsKeys[V any](m map[string]V) []string {
	out := []string{}
	for k := range m {
		out = append(out, k)
	}
	sort.Strings(out)
	return out
}

// basicsReadFile is path.read_text(): the test fails when it cannot.
func basicsReadFile(t *testing.T, path string) string {
	t.Helper()
	data, err := os.ReadFile(path)
	if err != nil {
		t.Fatal(err)
	}
	return string(data)
}

// basicsLoadObject is json.loads(text), a JSON object.
func basicsLoadObject(t *testing.T, text string) *pyjson.Object {
	t.Helper()
	v, err := pyjson.LoadsString(text)
	if err != nil {
		t.Fatal(err)
	}
	o, ok := v.(*pyjson.Object)
	if !ok {
		t.Fatalf("not a JSON object: %s", text)
	}
	return o
}

// basicsIntp is an *int, for the arguments the Python passes an int or None.
func basicsIntp(n int) *int { return &n }

// ------------------------------------------------------------------ ConfigTests

func TestBasics_CommittedConfigLoads(t *testing.T) {
	cfg := config.Load("", map[string]string{})
	basicsEqual(t, cfg.Repo, "jgoetzmann/JackiOh")
	first := cfg.Pool.Get("claude-1")
	basicsEqual(t, []string{first.ID, first.CLI, first.Model, first.Effort, first.Secret},
		[]string{"claude-1", "claude", "opus", "xhigh", "CLAUDE_CODE_OAUTH_TOKEN"})
	basicsEqual(t, []string{first.Schedule.Start, first.Schedule.End}, []string{"21:00", "07:00"})
	basicsEqual(t, first.OffHours, pyjson.New("five_hour", 0.4)) // and outside, up to 40%
	fourth := cfg.Pool.Get("claude-4")
	basicsEqual(t, []string{fourth.Schedule.Start, fourth.Schedule.End}, []string{"03:00", "15:00"})
	basicsEqual(t, fourth.Limits.Stops(), pyjson.New("five_hour", 0.7)) // no weekly cap
	basicsEqual(t, fourth.OffHours, pyjson.New("five_hour", 0.5))
	sixth := cfg.Pool.Get("claude-6")
	basicsEqual(t, sixth.Secret, "CLAUDE_CODE_OAUTH_TOKEN_6")
	basicsEqual(t, []string{sixth.Schedule.Start, sixth.Schedule.End}, []string{"03:00", "15:00"})
	basicsEqual(t, sixth.Limits.Stops(), pyjson.New("five_hour", 0.7)) // no weekly cap
	basicsEqual(t, sixth.OffHours, pyjson.New("five_hour", 0.5))
	firstCaps := cfg.Pool.Get("claude-1")
	basicsEqual(t, firstCaps.Limits.Stops(), pyjson.New("five_hour", 1.0)) // no weekly cap
	basicsEqual(t, cfg.MaxSelfCheckRounds, 3)
	basicsIn(t, "plan", cfg.MaxTurns)
	basicsEqual(t, [2]int{cfg.Pool.MaxParallel, cfg.Pool.MachineParallel}, [2]int{8, 6})
	basicsIn(t, ".github/", cfg.ForbiddenPaths)
	basicsIn(t, "bot/", cfg.ForbiddenPaths)
	basicsIn(t, ".harness/", cfg.ForbiddenPaths)
	basicsEqual(t, cfg.SuggestionsMaxOpen, 10)
	basicsIn(t, "bot selftest", cfg.RequiredChecks)
}

func TestBasics_UnknownAndMissingKeysAreErrors(t *testing.T) {
	raw := testsupport.RawConfig()
	raw.Set("surprise", 1)
	basicsRaises[*herr.ConfigError](t, func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) })
	raw = testsupport.RawConfig()
	raw.Delete("gates")
	basicsRaises[*herr.ConfigError](t, func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) })
}

func TestBasics_TheOldSingleSubscriptionKeysAreGone(t *testing.T) {
	for _, key := range []string{"model", "effort", "window", "usage_stop"} {
		raw := testsupport.RawConfig()
		raw.Set(key, 1)
		basicsRaises[*herr.ConfigError](t, func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) }, key)
	}
}

func TestBasics_EnvironmentSuppliesTokensAndRun(t *testing.T) {
	cfg := testsupport.MakeConfig(testsupport.Config{Env: map[string]string{
		"BOT_GITHUB_TOKEN": "bot-token-value", "GITHUB_TOKEN": "actions-token"}})
	basicsEqual(t, cfg.WriteToken(), "bot-token-value")
	basicsEqual(t, cfg.RunURL(), "https://github.com/jgoetzmann/JackiOh/actions/runs/777")
	cfg = testsupport.MakeConfig(testsupport.Config{Env: map[string]string{"GITHUB_TOKEN": "actions-token"}})
	basicsEqual(t, cfg.WriteToken(), "actions-token")
}

func TestBasics_ChildEnvStripsEverySecret(t *testing.T) {
	env := map[string]string{"PATH": "/bin", "BOT_GITHUB_TOKEN": "x", "GITHUB_TOKEN": "y",
		"CLAUDE_CODE_OAUTH_TOKEN": "z", "ANTHROPIC_API_KEY": "k"}
	basicsEqual(t, config.ChildEnv(env, nil), map[string]string{"PATH": "/bin"})
	kept := config.ChildEnv(env, []string{"CLAUDE_CODE_OAUTH_TOKEN"})
	basicsEqual(t, kept, map[string]string{"PATH": "/bin", "CLAUDE_CODE_OAUTH_TOKEN": "z"})
}

// ------------------------------------------------------------------ WindowTests

// basicsWindow is WindowTests.window.
var basicsWindow = clock.WindowOf("America/Chicago", "21:00", "07:00")

// basicsAt is WindowTests.at.
func basicsAt(text string) time.Time { return clock.ParseISO(text) }

func TestBasics_OpenAtNightClosedByDay(t *testing.T) {
	basicsTrue(t, basicsWindow.IsOpen(testsupport.Night))
	basicsFalse(t, basicsWindow.IsOpen(testsupport.Day))
}

func TestBasics_EdgesInSummerTime(t *testing.T) {
	// CDT is UTC-5: the window is 02:00 to 12:00 UTC.
	basicsFalse(t, basicsWindow.IsOpen(basicsAt("2026-07-01T01:59:00Z")))
	basicsTrue(t, basicsWindow.IsOpen(basicsAt("2026-07-01T02:00:00Z")))
	basicsTrue(t, basicsWindow.IsOpen(basicsAt("2026-07-01T11:59:00Z")))
	basicsFalse(t, basicsWindow.IsOpen(basicsAt("2026-07-01T12:00:00Z")))
}

func TestBasics_EdgesInWinterTime(t *testing.T) {
	// CST is UTC-6: the window is 03:00 to 13:00 UTC.
	basicsFalse(t, basicsWindow.IsOpen(basicsAt("2026-12-01T02:59:00Z")))
	basicsTrue(t, basicsWindow.IsOpen(basicsAt("2026-12-01T03:00:00Z")))
	basicsTrue(t, basicsWindow.IsOpen(basicsAt("2026-12-01T12:59:00Z")))
	basicsFalse(t, basicsWindow.IsOpen(basicsAt("2026-12-01T13:00:00Z")))
}

func TestBasics_NextOpenAndClose(t *testing.T) {
	opens := basicsWindow.NextOpen(testsupport.Day)
	basicsEqual(t, clock.ISO(opens), "2026-09-30T02:00:00Z")
	closes := basicsWindow.ClosesAt(testsupport.Night)
	basicsEqual(t, clock.ISO(closes), "2026-09-30T12:00:00Z")
	basicsTrue(t, basicsWindow.ClosesAt(testsupport.Day).IsZero()) // None
}

func TestBasics_FallbackZoneAgreesWithTzdata(t *testing.T) {
	fallback := &clock.UsCentral{}
	real := clock.Zone("America/Chicago")
	at := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	for range (24*400 + 6) / 7 {
		at = at.Add(7 * time.Hour)
		basicsEqual(t, fallback.Fromutc(at).Hour(), at.In(real).Hour(), at)
	}
}

func TestBasics_CronsCoverTheWindowInBothSeasons(t *testing.T) {
	// bot-night.yml fires at :17 every hour; every local hour 21..06 must be covered.
	for _, day := range []string{"2026-07-01", "2026-12-01"} {
		covered := map[int]bool{}
		for hour := range 24 {
			at := basicsAt(fmt.Sprintf("%sT%02d:17:00Z", day, hour))
			if basicsWindow.IsOpen(at) {
				covered[at.In(clock.Zone("America/Chicago")).Hour()] = true
			}
		}
		basicsEqual(t, covered, map[int]bool{21: true, 22: true, 23: true, 0: true, 1: true, 2: true,
			3: true, 4: true, 5: true, 6: true}, day)
	}
}

func TestBasics_HumanDelta(t *testing.T) {
	basicsEqual(t, clock.HumanDelta(5*time.Minute), "5m")
	basicsEqual(t, clock.HumanDelta(3*time.Hour+4*time.Minute), "3h 04m")
	basicsEqual(t, clock.HumanDelta(-5*time.Second), "now")
}

// ------------------------------------------------------------------ CommandTests

// basicsBot is CommandTests.bot.
const basicsBot = "jgoetzmann-bot"

// basicsVerb is (c.verb, c.args, c.force).
type basicsVerb struct {
	Verb  string
	Args  string
	Force bool
}

// basicsVerbs is CommandTests.verbs.
func basicsVerbs(body string) []basicsVerb {
	out := []basicsVerb{}
	for _, c := range commands.Parse(body, basicsBot) {
		out = append(out, basicsVerb{c.Verb, c.Args, c.Force})
	}
	return out
}

func TestBasics_SlashForms(t *testing.T) {
	basicsEqual(t, basicsVerbs("/harness build"), []basicsVerb{{"build", "", false}})
	basicsEqual(t, basicsVerbs("/harness-status"), []basicsVerb{{"status", "", false}})
	basicsEqual(t, basicsVerbs("/Harness HALT tired"), []basicsVerb{{"halt", "tired", false}})
	basicsEqual(t, basicsVerbs("@jgoetzmann-bot /harness revise tighten it"),
		[]basicsVerb{{"revise", "tighten it", false}})
}

func TestBasics_Aliases(t *testing.T) {
	basicsEqual(t, basicsVerbs("/harness work"), []basicsVerb{{"build", "", false}})
	basicsEqual(t, basicsVerbs("/harness fix it"), []basicsVerb{{"revise", "it", false}})
	basicsEqual(t, basicsVerbs("/harness resume"), []basicsVerb{{"start", "", false}})
	basicsEqual(t, basicsVerbs("/harness help"), []basicsVerb{{"help", "", false}})
}

func TestBasics_ForceFlag(t *testing.T) {
	basicsEqual(t, basicsVerbs("/harness build --force"), []basicsVerb{{"build", "", true}})
	found := commands.Parse("/harness build --force", basicsBot)[0]
	basicsEqual(t, found.Level(), 3)
	basicsEqual(t, commands.Parse("/harness build", basicsBot)[0].Level(), 2)
}

func TestBasics_MentionWithAVerbAndWithFreeText(t *testing.T) {
	basicsEqual(t, basicsVerbs("@jgoetzmann-bot status"), []basicsVerb{{"status", "", false}})
	basicsEqual(t, basicsVerbs("@jgoetzmann-bot: please make the coin shinier"),
		[]basicsVerb{{"request", "please make the coin shinier", false}})
	basicsEqual(t, basicsVerbs("@JGoetzmann-Bot build --force"), []basicsVerb{{"build", "", true}})
}

func TestBasics_ProseIsNotACommand(t *testing.T) {
	basicsEqual(t, basicsVerbs("as discussed, /harness stop"), []basicsVerb{})
	basicsEqual(t, basicsVerbs("thanks @jgoetzmann-bot, looks good"), []basicsVerb{})
	basicsEqual(t, basicsVerbs("@someone status"), []basicsVerb{})
}

func TestBasics_QuotesAndFencesAreSkipped(t *testing.T) {
	body := "> /harness halt\n```\n/harness halt\n```\n~~~\n@jgoetzmann-bot build\n~~~\n"
	basicsEqual(t, basicsVerbs(body), []basicsVerb{})
}

func TestBasics_AVoteIsTheWordAloneOnItsLine(t *testing.T) {
	// #760: `approve` and `reject` answer a suggestion, bare or after either prefix.
	vote := func(body string) [2]string {
		word, line := commands.Vote(body, basicsBot)
		return [2]string{word, line}
	}
	basicsEqual(t, vote("approve"), [2]string{"approve", "approve"})
	basicsEqual(t, vote("Thanks!\n- **Reject.**")[0], "reject")
	basicsEqual(t, vote("@jgoetzmann-bot: reject")[0], "reject")
	basicsEqual(t, vote("/harness approve")[0], "approve")
	for _, prose := range []string{"I approve of this", "reject it if the tests fail", "> approve",
		"```\napprove\n```", "@someone approve"} {
		basicsEqual(t, vote(prose), [2]string{"", ""}, prose)
	}
	basicsEqual(t, commands.Parse("@jgoetzmann-bot reject", basicsBot)[0].Line,
		vote("@jgoetzmann-bot reject")[1])
}

func TestBasics_SeveralCommandsInOrderAndCapped(t *testing.T) {
	lines := []string{}
	for range 8 {
		lines = append(lines, "/harness status", "/harness build")
	}
	found := commands.Parse(strings.Join(lines, "\n"), basicsBot)
	basicsEqual(t, len(found), commands.MaxCommands)
	basicsEqual(t, []string{found[0].Verb, found[1].Verb}, []string{"status", "build"})
}

func TestBasics_WordsThatAreNotAVerbAreARequestAfterEitherPrefix(t *testing.T) {
	basicsEqual(t, basicsVerbs("/harness dance"), []basicsVerb{{"request", "dance", false}})
	basicsEqual(t, basicsVerbs("/harness please make it shiny --force"),
		[]basicsVerb{{"request", "please make it shiny", true}})
	basicsEqual(t, basicsVerbs("/harness"), basicsVerbs("@jgoetzmann-bot"))
}

func TestBasics_BothPrefixesReadEveryVerbAndAliasTheSame(t *testing.T) {
	type read struct {
		Verb  string
		Args  string
		Force bool
		Meant string
	}
	lines := slices.Clone(commands.Verbs)
	lines = append(lines, commands.Aliases.Keys()...)
	lines = append(lines, "build make it spin", "revise tighten it", "run #12", "start --force",
		"help build", "please make the coin shinier", "stauts", "biuld --force")
	for _, line := range lines {
		forms := []string{"/harness " + line, "/harness-" + line, "@" + basicsBot + " " + line,
			"@" + basicsBot + " /harness " + line}
		reads := [][]read{}
		for _, f := range forms {
			one := []read{}
			for _, c := range commands.Parse(f, basicsBot) {
				one = append(one, read{c.Verb, c.Args, c.Force, c.Meant})
			}
			reads = append(reads, one)
		}
		ok := len(reads[0]) > 0
		for _, r := range reads {
			ok = ok && reflect.DeepEqual(r, reads[0])
		}
		basicsTrue(t, ok, fmt.Sprintf("%q %+v", line, reads))
	}
}

func TestBasics_AMisspeltVerbRunsNothing(t *testing.T) {
	meant := func(body string) [][2]string {
		out := [][2]string{}
		for _, c := range commands.Parse(body, basicsBot) {
			out = append(out, [2]string{c.Verb, c.Meant})
		}
		return out
	}
	basicsEqual(t, meant("@jgoetzmann-bot stauts"), [][2]string{{"typo", "status"}})
	basicsEqual(t, meant("/harness biuld --force"), [][2]string{{"typo", "build"}})
	basicsEqual(t, meant("/harness rnu #5"), [][2]string{{"typo", "run"}})
	basicsEqual(t, meant("@jgoetzmann-bot fit"), [][2]string{{"typo", "fix"}})
	basicsEqual(t, commands.Parse("/harness stauts", basicsBot)[0].Level(), 1)
	// More than one word is a request in plain words, near misses and all.
	basicsEqual(t, meant("@jgoetzmann-bot do the thing"), [][2]string{{"request", ""}})
	basicsEqual(t, meant("/harness sparkle"), [][2]string{{"request", ""}})
}

func TestBasics_AColonAfterAControlVerbMakesItsWordsItsOwn(t *testing.T) {
	basicsEqual(t, basicsVerbs("@jgoetzmann-bot halt: away this week"),
		[]basicsVerb{{"halt", "away this week", false}})
	basicsEqual(t, basicsVerbs("/harness halt: away this week"),
		[]basicsVerb{{"halt", "away this week", false}})
	basicsEqual(t, basicsVerbs("@jgoetzmann-bot halt away this week")[0].Verb, "request")
}

func TestBasics_HelpForOneVerb(t *testing.T) {
	text := commands.HelpText(basicsBot, "fix")
	basicsIn(t, "`revise <notes>`", text)
	basicsIn(t, "`fix`, `update`", text)
	basicsIn(t, "For example: `@jgoetzmann-bot revise", text)
	basicsIn(t, "I do not know `dance`", commands.HelpText(basicsBot, "dance"))
	basicsIn(t, "| `help [verb]` |", commands.HelpText(basicsBot, ""))
	basicsEqual(t, basicsSorted(commands.VerbHelp.Keys()), basicsSorted(commands.Verbs))
}

// Every verb the bot has; the night bot's one mode is `fullsend` (#505), so `oneshot` and `split`
// are Squishy's alone (#60).
func TestBasics_HelpMentionsEveryVerb(t *testing.T) {
	text := commands.HelpText(basicsBot, "")
	for _, verb := range commands.Verbs {
		if commands.Offered(verb) {
			basicsIn(t, "`"+verb, text)
		}
	}
	basicsNotIn(t, "`oneshot", text)
	basicsNotIn(t, "`split", text)
}

func TestBasics_HelpSaysWhoseAnIssueIs(t *testing.T) {
	// #802: an issue is the bot's or people's (`human`, or the bot's `human required`).
	text := commands.HelpText(basicsBot, "")
	basicsIn(t, "`human` keeps me off it", text)
	basicsIn(t, "`human required` is my hand-over when I find I cannot do it", text)
	basicsIn(t, "Take either label off to hand it back, then `/harness build`.", text)
}

func TestBasics_HelpListsTheVotesOnASuggestion(t *testing.T) {
	// #796: `approve` and `reject` (#760) are not verbs, but `help` names them.
	text := commands.HelpText(basicsBot, "")
	basicsIn(t, "| `approve`, `reject` |", text)
	for _, word := range []string{"approve", "Reject"} {
		one := commands.HelpText(basicsBot, word)
		basicsIn(t, "**`approve`** or **`reject`**", one)
		basicsIn(t, "`bot:suggestion`", one)
		basicsNotIn(t, "I do not know", one)
	}
}

// ------------------------------------------------------------------ TrustTests

// basicsTrustText is TrustTests.text.
const basicsTrustText = "jgoetzmann 3 id:95732896\nhelper 2\nreader\nbad!login 2\nhigh 9\n"

func TestBasics_Parse(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	basicsEqual(t, basicsKeys(trusted.Entries), []string{"helper", "jgoetzmann", "reader"})
	basicsEqual(t, len(trusted.Problems), 2)
}

func TestBasics_IDPinsTheAccount(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	basicsEqual(t, trusted.Level("jgoetzmann", basicsIntp(95732896), ""), 3)
	basicsEqual(t, trusted.Level("jgoetzmann", basicsIntp(1), ""), 0)
	basicsEqual(t, trusted.Level("JGoetzmann", basicsIntp(95732896), "NONE"), 3)
}

func TestBasics_APinnedLineNeedsTheID(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	basicsEqual(t, trusted.Level("jgoetzmann", nil, "OWNER"), 0)
}

func TestBasics_WithoutIDGithubMustVouch(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	basicsEqual(t, trusted.Level("helper", basicsIntp(42), "COLLABORATOR"), 2)
	basicsEqual(t, trusted.Level("helper", basicsIntp(42), "CONTRIBUTOR"), 0)
	basicsEqual(t, trusted.Level("reader", nil, "OWNER"), 1)
	basicsEqual(t, trusted.Level("nobody", basicsIntp(1), "OWNER"), 0)
}

func TestBasics_CommittedTrustFile(t *testing.T) {
	trusted := trust.TrustLoad(filepath.Join(config.RepoRoot, config.TrustPath))
	basicsEqual(t, trusted.Problems, []string{})
	basicsEqual(t, trusted.Level("jgoetzmann", basicsIntp(95732896), ""), 3)
}

// ------------------------------------------------------------------ RedactTests

func TestBasics_TokenShapes(t *testing.T) {
	text := "ghp_" + strings.Repeat("a", 36) + " github_pat_" + strings.Repeat("b", 50) + " sk-ant-" +
		strings.Repeat("c", 30) + " Bearer abcdefghijklmnopqrstuvwxyz"
	out := redact.Redact(text, nil)
	basicsNotIn(t, "aaaa", out)
	basicsNotIn(t, "bbbb", out)
	basicsNotIn(t, "cccc", out)
	basicsNotIn(t, "abcdefghijkl", out)
}

func TestBasics_LiveValues(t *testing.T) {
	basicsEqual(t, redact.Redact("key=hunter2hunter2", []string{"hunter2hunter2"}), "key=[REDACTED]")
}

// ------------------------------------------------------------------ VerdictTests

func TestBasics_BuildReport(t *testing.T) {
	text := "Some preamble\n<!-- bot: {\"status\": \"done\", \"title\": \"Fix the  coin\"} -->\n## What\nStuff"
	report := verdicts.BuildReport_(text)
	basicsEqual(t, []string{report.Status, report.Title}, []string{"done", "Fix the coin"})
	basicsTrue(t, strings.HasPrefix(report.Body, "## What"))
	blocked := verdicts.BuildReport_(`<!-- bot: {"status": "blocked", "question": "A or B?"} -->`)
	basicsEqual(t, []string{blocked.Status, blocked.Question}, []string{"blocked", "A or B?"})
	basicsEqual(t, verdicts.BuildReport_("no header").Status, "unknown")
}

func TestBasics_Review(t *testing.T) {
	text := `<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", ` +
		`"where": "a.ts:3", "claim": "wrong", "evidence": "x"}, {"severity": "note", ` +
		`"where": "b.ts", "claim": "meh"}]} -->` + "\nprose"
	review := verdicts.Review_(text)
	basicsTrue(t, review.Readable)
	basicsFalse(t, review.Approved())
	basicsEqual(t, len(review.Blocking()), 1)
	basicsEqual(t, len(review.Notes()), 1)
	basicsEqual(t, review.Body, "prose")
}

func TestBasics_ApproveWithABlockingFindingIsNotApproval(t *testing.T) {
	text := `<!-- review: {"verdict": "approve", "findings": [{"severity": "blocking", ` +
		`"where": "x", "claim": "y"}]} -->`
	basicsFalse(t, verdicts.Review_(text).Approved())
	basicsTrue(t, verdicts.Review_(`<!-- review: {"verdict": "approve", "findings": []} -->`).Approved())
}

func TestBasics_LiteralNewlinesInsideJSONStrings(t *testing.T) {
	text := `<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", ` +
		`"where": "a.ts", "claim": "x", "evidence": "line one` + "\n" + `line two"}]} -->`
	basicsTrue(t, verdicts.Review_(text).Readable)
	text = `<!-- suggestions: [{"title": "T", "body": "## Why` + "\n" + `Because"}] -->`
	basicsEqual(t, len(verdicts.Suggestions(text, 4)), 1)
}

func TestBasics_ChangesWithNothingBlockingIsApproval(t *testing.T) {
	text := `<!-- review: {"verdict": "changes", "findings": [{"severity": "note", ` +
		`"where": "a", "claim": "b"}]} -->`
	basicsTrue(t, verdicts.Review_(text).Approved())
}

func TestBasics_UnreadableReview(t *testing.T) {
	basicsFalse(t, verdicts.Review_("LGTM!").Readable)
	basicsFalse(t, verdicts.Review_(`<!-- review: {"verdict": "maybe"} -->`).Readable)
	basicsFalse(t, verdicts.Review_("<!-- review: {not json} -->").Approved())
}

func TestBasics_Suggestions(t *testing.T) {
	text := `<!-- suggestions: [{"title": "A", "body": "B"}, {"title": "", "body": "x"}, ` +
		`{"title": "C", "body": "D"}, {"title": "E", "body": "F"}] -->`
	found := verdicts.Suggestions(text, 2)
	titles := []string{}
	for _, s := range found {
		titles = append(titles, s.Title)
	}
	basicsEqual(t, titles, []string{"A", "C"})
	basicsEqual(t, verdicts.Suggestions("nothing", 4), []*verdicts.Suggestion{})
}

// ------------------------------------------------------------------ PromptTests

func TestBasics_EveryTemplateRenders(t *testing.T) {
	basicsEqual(t, work.CheckTemplates(), []string{})
}

func TestBasics_DataFenceOutgrowsBackticksInside(t *testing.T) {
	wrapped := prompts.Data("before ```` after", "Thing")
	basicsIn(t, "`````text", wrapped)
	basicsIn(t, "data, not instructions", wrapped)
}

func TestBasics_SystemPromptNamesTheForbiddenPaths(t *testing.T) {
	text, err := prompts.Render("system", config.PromptsDir, pyjson.New("bot", "b", "repo", "r",
		"title", "night bot", "who", "the night bot"))
	if err != nil {
		t.Fatal(err)
	}
	for _, path := range []string{".github/", ".harness/", ".squishy/", "bot/"} {
		basicsIn(t, path, text)
	}
}

// ------------------------------------------------------------------ StateTests

// basicsSleep is time.sleep, the state store's default.
func basicsSleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

// basicsStore is state.StateStore(gh), with its defaults.
func basicsStore(gh *fakes.FakeGitHub) *state.StateStore {
	return state.NewStateStore(gh, config.StateBranch, config.StateFile, basicsSleep)
}

func TestBasics_UpdateCreatesTheBranchAndRetriesOnConflict(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	store := basicsStore(gh)
	store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "halt")
	basicsIn(t, "bot-state", gh.Branches)
	basicsTrue(t, pyjson.Truthy(store.Load().Get("halted")))
	gh.ConflictsToInject = 2
	store.Update(func(s *pyjson.Object) any { state.Item(s, 5).Set("failures", 1); return nil }, "fail")
	loaded := store.Load()
	basicsEqual(t, loaded.Obj("items").Obj("5").Get("failures"), 1)
	basicsIn(t, "999", loaded.Obj("items")) // the other writer's change survived
	basicsTrue(t, pyjson.Truthy(loaded.Get("halted")))
}

func TestBasics_UpdateGivesUpEventually(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	store := basicsStore(gh)
	store.Update(func(s *pyjson.Object) any { s.Set("halted", false); return nil }, "x")
	gh.ConflictsToInject = 99
	basicsRaises[*herr.StateConflict](t, func() {
		store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "y")
	})
}

// On 2026-10-25 the state read as empty and the next write replaced all of it.
func TestBasics_AStateThatIsNotJSONIsNeverWrittenOver(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	store := basicsStore(gh)
	store.Update(func(s *pyjson.Object) any { s.Set("halted", true); return nil }, "x")
	for _, text := range []string{"", `{"items": {"5": `} {
		gh.Files[[2]string{"bot-state", "state.json"}] = [2]string{text, "sha-kept"}
		basicsRaises[*herr.StateUnreadable](t, func() { store.Load() })
		basicsRaises[*herr.StateUnreadable](t, func() {
			store.Update(func(s *pyjson.Object) any { s.Set("halted", false); return nil }, "y")
		})
		basicsEqual(t, gh.Files[[2]string{"bot-state", "state.json"}], [2]string{text, "sha-kept"})
	}
}

// Over 1 MB the contents API answers with an empty `content`; state.json passed that on
// 2026-10-25, read as empty and was written over.
func TestBasics_AFileOver1MbIsReadFromItsBlob(t *testing.T) {
	big := pyjson.Dumps(pyjson.New("items", pyjson.New("5", pyjson.New("note", strings.Repeat("x", 1_100_000)))))
	asked := []string{}
	opener := func(req *ghpkg.Request, timeout time.Duration) (*ghpkg.Response, error) {
		asked = append(asked, req.URL)
		var body *pyjson.Object
		if strings.Contains(req.URL, "/contents/") {
			body = pyjson.New("type", "file", "encoding", "none", "content", "", "size", len(big),
				"sha", "abc123")
		} else {
			body = pyjson.New("sha", "abc123", "encoding", "base64", "size", len(big),
				"content", base64.StdEncoding.EncodeToString([]byte(big)))
		}
		return &ghpkg.Response{Body: []byte(pyjson.Dumps(body))}, nil
	}
	text, sha := ghpkg.NewGitHub("o/r", "token", false, opener, basicsSleep, "").GetFile("state.json", "bot-state")
	basicsEqual(t, [2]string{text, sha}, [2]string{big, "abc123"})
	basicsTrue(t, strings.HasSuffix(asked[len(asked)-1], "/repos/o/r/git/blobs/abc123"), asked)
}

// ------------------------------------------------------------------ PathTests

func TestBasics_ForbiddenAndReviewPaths(t *testing.T) {
	cfg := testsupport.MakeConfig()
	forbidden, review := cfg.ForbiddenPaths, cfg.ReviewPaths
	for _, path := range []string{".github/workflows/x.yml", ".GitHub/workflows/x.yml", "./bot/harness/gh.py",
		".harness/trust.txt", ".claude/settings.json", ".mcp.json", ".vscode/tasks.json", ".github"} {
		basicsTrue(t, git.Matches(path, forbidden), path)
	}
	for _, path := range []string{"package.json", "apps/web/package.json", "pnpm-lock.yaml",
		"Cargo.toml", "crates/engine/Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
		"rustfmt.toml", "crates/engine/clippy.toml", "crates/cards/build.rs",
		".cargo/config.toml", "vitest.config.ts", "apps/web/tsconfig.json",
		"eslint.config.js", "scripts/build-wasm.sh", "render.yaml",
		"crates/server/Dockerfile", "crates/server/migrations/0005_x.sql",
		"e2e/cypress.config.ts"} {
		basicsTrue(t, git.Matches(path, review), path)
	}
	for _, path := range []string{"apps/web/src/game/Game.tsx", "crates/cards/src/scripts/core/c002_bigot.rs",
		"SPEC.md", "bots/x", "docs/scripts.md", "crates/engine/tests/rules/combat.rs"} {
		basicsFalse(t, git.Matches(path, forbidden), path)
		basicsFalse(t, git.Matches(path, review), path)
	}
}

func TestBasics_AnExceptionTakesAPathBackOut(t *testing.T) {
	basicsFalse(t, git.Matches(".claude/skills/x/SKILL.md", []string{".claude/", "!.claude/skills/"}))
	basicsTrue(t, git.Matches(".claude/settings.json", []string{".claude/", "!.claude/skills/"}))
	basicsTrue(t, git.Matches(".claude/skillset.json", []string{".claude/", "!.claude/skills/"}))
	basicsFalse(t, git.Matches("x.md", []string{"!x.md"}))
	// Squishy may change the skills (the weekly hygiene issue, #839); the night bot may not, and
	// neither may change the rest of .claude/.
	for _, c := range []struct {
		home            string
		skillsForbidden bool
	}{{".harness", true}, {".squishy", false}} {
		raw := basicsLoadObject(t, basicsReadFile(t, filepath.Join(config.RepoRoot, c.home, "config.json")))
		forbidden := pyjson.Strings(raw.Get("forbidden_paths"))
		basicsEqual(t, git.Matches(".claude/skills/maxi-tail/SKILL.md", forbidden), c.skillsForbidden, c.home)
		for _, path := range []string{".claude/settings.json", ".claude/settings.local.json", ".claude/hooks/x.sh"} {
			basicsTrue(t, git.Matches(path, forbidden), c.home, " ", path)
		}
	}
}
