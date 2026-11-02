// basics_test.go is bot/tests/test_basics.py: config, clock and window, commands, trust,
// redaction, verdicts, prompts, state.
package tests

import (
	"encoding/base64"
	"fmt"
	"path/filepath"
	"reflect"
	"slices"
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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/state"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/trust"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// ------------------------------------------------------------------ ConfigTests

func TestBasics_CommittedConfigLoads(t *testing.T) {
	cfg := config.Load("", map[string]string{})
	assertEqual(t, cfg.Repo, "jgoetzmann/JackiOh")
	first := cfg.Pool.Get("claude-1")
	assertEqual(t, []string{first.ID, first.CLI, first.Model, first.Effort, first.Secret},
		[]string{"claude-1", "claude", "opus", "xhigh", "CLAUDE_CODE_OAUTH_TOKEN"})
	assertEqual(t, []string{first.Schedule.Start, first.Schedule.End}, []string{"21:00", "07:00"})
	assertEqual(t, first.OffHours, pyjson.New("five_hour", 0.4)) // and outside, up to 40%
	fourth := cfg.Pool.Get("claude-4")
	assertEqual(t, []string{fourth.Schedule.Start, fourth.Schedule.End}, []string{"03:00", "15:00"})
	assertEqual(t, fourth.Limits.Stops(), pyjson.New("five_hour", 0.7)) // no weekly cap
	assertEqual(t, fourth.OffHours, pyjson.New("five_hour", 0.5))
	sixth := cfg.Pool.Get("claude-6")
	assertEqual(t, sixth.Secret, "CLAUDE_CODE_OAUTH_TOKEN_6")
	assertEqual(t, []string{sixth.Schedule.Start, sixth.Schedule.End}, []string{"03:00", "15:00"})
	assertEqual(t, sixth.Limits.Stops(), pyjson.New("five_hour", 0.7)) // no weekly cap
	assertEqual(t, sixth.OffHours, pyjson.New("five_hour", 0.5))
	firstCaps := cfg.Pool.Get("claude-1")
	assertEqual(t, firstCaps.Limits.Stops(), pyjson.New("five_hour", 1.0)) // no weekly cap
	assertEqual(t, cfg.MaxSelfCheckRounds, 3)
	assertIn(t, "plan", cfg.MaxTurns)
	assertEqual(t, [2]int{cfg.Pool.MaxParallel, cfg.Pool.MachineParallel}, [2]int{8, 6})
	assertIn(t, ".github/", cfg.ForbiddenPaths)
	assertIn(t, "bot/", cfg.ForbiddenPaths)
	assertIn(t, ".harness/", cfg.ForbiddenPaths)
	assertEqual(t, cfg.SuggestionsMaxOpen, 10)
	assertIn(t, "bot selftest", cfg.RequiredChecks)
}

func TestBasics_UnknownAndMissingKeysAreErrors(t *testing.T) {
	raw := testsupport.RawConfig()
	raw.Set("surprise", 1)
	assertRaises[*herr.ConfigError](t, func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) })
	raw = testsupport.RawConfig()
	raw.Delete("gates")
	assertRaises[*herr.ConfigError](t, func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) })
}

func TestBasics_TheOldSingleSubscriptionKeysAreGone(t *testing.T) {
	for _, key := range []string{"model", "effort", "window", "usage_stop"} {
		raw := testsupport.RawConfig()
		raw.Set(key, 1)
		assertRaises[*herr.ConfigError](t, func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) }, key)
	}
}

func TestBasics_EnvironmentSuppliesTokensAndRun(t *testing.T) {
	cfg := testsupport.MakeConfig(testsupport.Config{Env: map[string]string{
		"BOT_GITHUB_TOKEN": "bot-token-value", "GITHUB_TOKEN": "actions-token"}})
	assertEqual(t, cfg.WriteToken(), "bot-token-value")
	assertEqual(t, cfg.RunURL(), "https://github.com/jgoetzmann/JackiOh/actions/runs/777")
	cfg = testsupport.MakeConfig(testsupport.Config{Env: map[string]string{"GITHUB_TOKEN": "actions-token"}})
	assertEqual(t, cfg.WriteToken(), "actions-token")
}

func TestBasics_ChildEnvStripsEverySecret(t *testing.T) {
	env := map[string]string{"PATH": "/bin", "BOT_GITHUB_TOKEN": "x", "GITHUB_TOKEN": "y",
		"CLAUDE_CODE_OAUTH_TOKEN": "z", "ANTHROPIC_API_KEY": "k"}
	assertEqual(t, config.ChildEnv(env, nil), map[string]string{"PATH": "/bin"})
	kept := config.ChildEnv(env, []string{"CLAUDE_CODE_OAUTH_TOKEN"})
	assertEqual(t, kept, map[string]string{"PATH": "/bin", "CLAUDE_CODE_OAUTH_TOKEN": "z"})
}

// ------------------------------------------------------------------ WindowTests

// basicsWindow is WindowTests.window.
var basicsWindow = clock.WindowOf("America/Chicago", "21:00", "07:00")

// basicsAt is WindowTests.at.
func basicsAt(text string) time.Time { return clock.ParseISO(text) }

func TestBasics_OpenAtNightClosedByDay(t *testing.T) {
	assertTrue(t, basicsWindow.IsOpen(testsupport.Night))
	assertFalse(t, basicsWindow.IsOpen(testsupport.Day))
}

func TestBasics_EdgesInSummerTime(t *testing.T) {
	// CDT is UTC-5: the window is 02:00 to 12:00 UTC.
	assertFalse(t, basicsWindow.IsOpen(basicsAt("2026-07-01T01:59:00Z")))
	assertTrue(t, basicsWindow.IsOpen(basicsAt("2026-07-01T02:00:00Z")))
	assertTrue(t, basicsWindow.IsOpen(basicsAt("2026-07-01T11:59:00Z")))
	assertFalse(t, basicsWindow.IsOpen(basicsAt("2026-07-01T12:00:00Z")))
}

func TestBasics_EdgesInWinterTime(t *testing.T) {
	// CST is UTC-6: the window is 03:00 to 13:00 UTC.
	assertFalse(t, basicsWindow.IsOpen(basicsAt("2026-12-01T02:59:00Z")))
	assertTrue(t, basicsWindow.IsOpen(basicsAt("2026-12-01T03:00:00Z")))
	assertTrue(t, basicsWindow.IsOpen(basicsAt("2026-12-01T12:59:00Z")))
	assertFalse(t, basicsWindow.IsOpen(basicsAt("2026-12-01T13:00:00Z")))
}

func TestBasics_NextOpenAndClose(t *testing.T) {
	opens := basicsWindow.NextOpen(testsupport.Day)
	assertEqual(t, clock.ISO(opens), "2026-09-30T02:00:00Z")
}

func TestBasics_FallbackZoneAgreesWithTzdata(t *testing.T) {
	fallback := &clock.UsCentral{}
	real := clock.Zone("America/Chicago")
	at := time.Date(2026, 1, 1, 0, 0, 0, 0, time.UTC)
	for range (24*400 + 6) / 7 {
		at = at.Add(7 * time.Hour)
		assertEqual(t, fallback.Fromutc(at).Hour(), at.In(real).Hour(), at)
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
		assertEqual(t, covered, map[int]bool{21: true, 22: true, 23: true, 0: true, 1: true, 2: true,
			3: true, 4: true, 5: true, 6: true}, day)
	}
}

func TestBasics_HumanDelta(t *testing.T) {
	assertEqual(t, clock.HumanDelta(5*time.Minute), "5m")
	assertEqual(t, clock.HumanDelta(3*time.Hour+4*time.Minute), "3h 04m")
	assertEqual(t, clock.HumanDelta(-5*time.Second), "now")
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
	assertEqual(t, basicsVerbs("/harness build"), []basicsVerb{{"build", "", false}})
	assertEqual(t, basicsVerbs("/harness-status"), []basicsVerb{{"status", "", false}})
	assertEqual(t, basicsVerbs("/Harness HALT tired"), []basicsVerb{{"halt", "tired", false}})
	assertEqual(t, basicsVerbs("@jgoetzmann-bot /harness revise tighten it"),
		[]basicsVerb{{"revise", "tighten it", false}})
}

func TestBasics_Aliases(t *testing.T) {
	assertEqual(t, basicsVerbs("/harness work"), []basicsVerb{{"build", "", false}})
	assertEqual(t, basicsVerbs("/harness fix it"), []basicsVerb{{"revise", "it", false}})
	assertEqual(t, basicsVerbs("/harness resume"), []basicsVerb{{"start", "", false}})
	assertEqual(t, basicsVerbs("/harness help"), []basicsVerb{{"help", "", false}})
}

func TestBasics_ForceFlag(t *testing.T) {
	assertEqual(t, basicsVerbs("/harness build --force"), []basicsVerb{{"build", "", true}})
	found := commands.Parse("/harness build --force", basicsBot)[0]
	assertEqual(t, found.Level(), 3)
	assertEqual(t, commands.Parse("/harness build", basicsBot)[0].Level(), 2)
}

func TestBasics_MentionWithAVerbAndWithFreeText(t *testing.T) {
	assertEqual(t, basicsVerbs("@jgoetzmann-bot status"), []basicsVerb{{"status", "", false}})
	assertEqual(t, basicsVerbs("@jgoetzmann-bot: please make the coin shinier"),
		[]basicsVerb{{"request", "please make the coin shinier", false}})
	assertEqual(t, basicsVerbs("@JGoetzmann-Bot build --force"), []basicsVerb{{"build", "", true}})
}

func TestBasics_ProseIsNotACommand(t *testing.T) {
	assertEqual(t, basicsVerbs("as discussed, /harness stop"), []basicsVerb{})
	assertEqual(t, basicsVerbs("thanks @jgoetzmann-bot, looks good"), []basicsVerb{})
	assertEqual(t, basicsVerbs("@someone status"), []basicsVerb{})
}

func TestBasics_QuotesAndFencesAreSkipped(t *testing.T) {
	body := "> /harness halt\n```\n/harness halt\n```\n~~~\n@jgoetzmann-bot build\n~~~\n"
	assertEqual(t, basicsVerbs(body), []basicsVerb{})
}

func TestBasics_AVoteIsTheWordAloneOnItsLine(t *testing.T) {
	// #760: `approve` and `reject` answer a suggestion, bare or after either prefix.
	vote := func(body string) [2]string {
		word, line := commands.Vote(body, basicsBot)
		return [2]string{word, line}
	}
	assertEqual(t, vote("approve"), [2]string{"approve", "approve"})
	assertEqual(t, vote("Thanks!\n- **Reject.**")[0], "reject")
	assertEqual(t, vote("@jgoetzmann-bot: reject")[0], "reject")
	assertEqual(t, vote("/harness approve")[0], "approve")
	for _, prose := range []string{"I approve of this", "reject it if the tests fail", "> approve",
		"```\napprove\n```", "@someone approve"} {
		assertEqual(t, vote(prose), [2]string{"", ""}, prose)
	}
	assertEqual(t, commands.Parse("@jgoetzmann-bot reject", basicsBot)[0].Line,
		vote("@jgoetzmann-bot reject")[1])
}

func TestBasics_SeveralCommandsInOrderAndCapped(t *testing.T) {
	lines := []string{}
	for range 8 {
		lines = append(lines, "/harness status", "/harness build")
	}
	found := commands.Parse(strings.Join(lines, "\n"), basicsBot)
	assertEqual(t, len(found), commands.MaxCommands)
	assertEqual(t, []string{found[0].Verb, found[1].Verb}, []string{"status", "build"})
}

func TestBasics_WordsThatAreNotAVerbAreARequestAfterEitherPrefix(t *testing.T) {
	assertEqual(t, basicsVerbs("/harness dance"), []basicsVerb{{"request", "dance", false}})
	assertEqual(t, basicsVerbs("/harness please make it shiny --force"),
		[]basicsVerb{{"request", "please make it shiny", true}})
	assertEqual(t, basicsVerbs("/harness"), basicsVerbs("@jgoetzmann-bot"))
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
		assertTrue(t, ok, fmt.Sprintf("%q %+v", line, reads))
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
	assertEqual(t, meant("@jgoetzmann-bot stauts"), [][2]string{{"typo", "status"}})
	assertEqual(t, meant("/harness biuld --force"), [][2]string{{"typo", "build"}})
	assertEqual(t, meant("/harness rnu #5"), [][2]string{{"typo", "run"}})
	assertEqual(t, meant("@jgoetzmann-bot fit"), [][2]string{{"typo", "fix"}})
	assertEqual(t, commands.Parse("/harness stauts", basicsBot)[0].Level(), 1)
	// More than one word is a request in plain words, near misses and all.
	assertEqual(t, meant("@jgoetzmann-bot do the thing"), [][2]string{{"request", ""}})
	assertEqual(t, meant("/harness sparkle"), [][2]string{{"request", ""}})
}

func TestBasics_AColonAfterAControlVerbMakesItsWordsItsOwn(t *testing.T) {
	assertEqual(t, basicsVerbs("@jgoetzmann-bot halt: away this week"),
		[]basicsVerb{{"halt", "away this week", false}})
	assertEqual(t, basicsVerbs("/harness halt: away this week"),
		[]basicsVerb{{"halt", "away this week", false}})
	assertEqual(t, basicsVerbs("@jgoetzmann-bot halt away this week")[0].Verb, "request")
}

func TestBasics_HelpForOneVerb(t *testing.T) {
	text := commands.HelpText(basicsBot, "fix")
	assertIn(t, "`revise <notes>`", text)
	assertIn(t, "`fix`, `update`", text)
	assertIn(t, "For example: `@jgoetzmann-bot revise", text)
	assertIn(t, "I do not know `dance`", commands.HelpText(basicsBot, "dance"))
	assertIn(t, "| `help [verb]` |", commands.HelpText(basicsBot, ""))
	assertEqual(t, sortedOf(commands.VerbHelp.Keys()), sortedOf(commands.Verbs))
}

// Every verb the bot has; the night bot's one mode is `fullsend` (#505), so `oneshot` and `split`
// are Squishy's alone (#60).
func TestBasics_HelpMentionsEveryVerb(t *testing.T) {
	text := commands.HelpText(basicsBot, "")
	for _, verb := range commands.Verbs {
		if commands.Offered(verb) {
			assertIn(t, "`"+verb, text)
		}
	}
	assertNotIn(t, "`oneshot", text)
	assertNotIn(t, "`split", text)
}

func TestBasics_HelpSaysWhoseAnIssueIs(t *testing.T) {
	// #802: an issue is the bot's or people's (`human`, or the bot's `human required`).
	text := commands.HelpText(basicsBot, "")
	assertIn(t, "`human` keeps me off it", text)
	assertIn(t, "`human required` is my hand-over when I find I cannot do it", text)
	assertIn(t, "Take either label off to hand it back, then `/harness build`.", text)
}

func TestBasics_HelpListsTheVotesOnASuggestion(t *testing.T) {
	// #796: `approve` and `reject` (#760) are not verbs, but `help` names them.
	text := commands.HelpText(basicsBot, "")
	assertIn(t, "| `approve`, `reject` |", text)
	for _, word := range []string{"approve", "Reject"} {
		one := commands.HelpText(basicsBot, word)
		assertIn(t, "**`approve`** or **`reject`**", one)
		assertIn(t, "`bot:suggestion`", one)
		assertNotIn(t, "I do not know", one)
	}
}

// ------------------------------------------------------------------ TrustTests

// basicsTrustText is TrustTests.text.
const basicsTrustText = "jgoetzmann 3 id:95732896\nhelper 2\nreader\nbad!login 2\nhigh 9\n"

func TestBasics_Parse(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	assertEqual(t, sortedKeys(trusted.Entries), []string{"helper", "jgoetzmann", "reader"})
	assertEqual(t, len(trusted.Problems), 2)
}

func TestBasics_IDPinsTheAccount(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	assertEqual(t, trusted.Level("jgoetzmann", ptr(95732896), ""), 3)
	assertEqual(t, trusted.Level("jgoetzmann", ptr(1), ""), 0)
	assertEqual(t, trusted.Level("JGoetzmann", ptr(95732896), "NONE"), 3)
}

func TestBasics_APinnedLineNeedsTheID(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	assertEqual(t, trusted.Level("jgoetzmann", nil, "OWNER"), 0)
}

func TestBasics_WithoutIDGithubMustVouch(t *testing.T) {
	trusted := trust.TrustParse(basicsTrustText)
	assertEqual(t, trusted.Level("helper", ptr(42), "COLLABORATOR"), 2)
	assertEqual(t, trusted.Level("helper", ptr(42), "CONTRIBUTOR"), 0)
	assertEqual(t, trusted.Level("reader", nil, "OWNER"), 1)
	assertEqual(t, trusted.Level("nobody", ptr(1), "OWNER"), 0)
}

func TestBasics_CommittedTrustFile(t *testing.T) {
	trusted := trust.TrustLoad(filepath.Join(config.RepoRoot, config.TrustPath))
	assertEqual(t, trusted.Problems, []string{})
	assertEqual(t, trusted.Level("jgoetzmann", ptr(95732896), ""), 3)
}

// ------------------------------------------------------------------ RedactTests

func TestBasics_TokenShapes(t *testing.T) {
	text := "ghp_" + strings.Repeat("a", 36) + " github_pat_" + strings.Repeat("b", 50) + " sk-ant-" +
		strings.Repeat("c", 30) + " Bearer abcdefghijklmnopqrstuvwxyz"
	out := redact.Redact(text, nil)
	assertNotIn(t, "aaaa", out)
	assertNotIn(t, "bbbb", out)
	assertNotIn(t, "cccc", out)
	assertNotIn(t, "abcdefghijkl", out)
}

func TestBasics_LiveValues(t *testing.T) {
	assertEqual(t, redact.Redact("key=hunter2hunter2", []string{"hunter2hunter2"}), "key=[REDACTED]")
}

// ------------------------------------------------------------------ VerdictTests

func TestBasics_BuildReport(t *testing.T) {
	text := "Some preamble\n<!-- bot: {\"status\": \"done\", \"title\": \"Fix the  coin\"} -->\n## What\nStuff"
	report := verdicts.BuildReport_(text)
	assertEqual(t, []string{report.Status, report.Title}, []string{"done", "Fix the coin"})
	assertTrue(t, strings.HasPrefix(report.Body, "## What"))
	blocked := verdicts.BuildReport_(`<!-- bot: {"status": "blocked", "question": "A or B?"} -->`)
	assertEqual(t, []string{blocked.Status, blocked.Question}, []string{"blocked", "A or B?"})
	assertEqual(t, verdicts.BuildReport_("no header").Status, "unknown")
}

func TestBasics_Review(t *testing.T) {
	text := `<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", ` +
		`"where": "a.ts:3", "claim": "wrong", "evidence": "x"}, {"severity": "note", ` +
		`"where": "b.ts", "claim": "meh"}]} -->` + "\nprose"
	review := verdicts.Review_(text)
	assertTrue(t, review.Readable)
	assertFalse(t, review.Approved())
	assertEqual(t, len(review.Blocking()), 1)
	assertEqual(t, len(review.Notes()), 1)
	assertEqual(t, review.Body, "prose")
}

func TestBasics_ApproveWithABlockingFindingIsNotApproval(t *testing.T) {
	text := `<!-- review: {"verdict": "approve", "findings": [{"severity": "blocking", ` +
		`"where": "x", "claim": "y"}]} -->`
	assertFalse(t, verdicts.Review_(text).Approved())
	assertTrue(t, verdicts.Review_(`<!-- review: {"verdict": "approve", "findings": []} -->`).Approved())
}

func TestBasics_LiteralNewlinesInsideJSONStrings(t *testing.T) {
	text := `<!-- review: {"verdict": "changes", "findings": [{"severity": "blocking", ` +
		`"where": "a.ts", "claim": "x", "evidence": "line one` + "\n" + `line two"}]} -->`
	assertTrue(t, verdicts.Review_(text).Readable)
	text = `<!-- suggestions: [{"title": "T", "body": "## Why` + "\n" + `Because"}] -->`
	assertEqual(t, len(verdicts.Suggestions(text, 4)), 1)
}

func TestBasics_ChangesWithNothingBlockingIsApproval(t *testing.T) {
	text := `<!-- review: {"verdict": "changes", "findings": [{"severity": "note", ` +
		`"where": "a", "claim": "b"}]} -->`
	assertTrue(t, verdicts.Review_(text).Approved())
}

func TestBasics_UnreadableReview(t *testing.T) {
	assertFalse(t, verdicts.Review_("LGTM!").Readable)
	assertFalse(t, verdicts.Review_(`<!-- review: {"verdict": "maybe"} -->`).Readable)
	assertFalse(t, verdicts.Review_("<!-- review: {not json} -->").Approved())
}

func TestBasics_Suggestions(t *testing.T) {
	text := `<!-- suggestions: [{"title": "A", "body": "B"}, {"title": "", "body": "x"}, ` +
		`{"title": "C", "body": "D"}, {"title": "E", "body": "F"}] -->`
	found := verdicts.Suggestions(text, 2)
	titles := []string{}
	for _, s := range found {
		titles = append(titles, s.Title)
	}
	assertEqual(t, titles, []string{"A", "C"})
	assertEqual(t, verdicts.Suggestions("nothing", 4), []*verdicts.Suggestion{})
}

// ------------------------------------------------------------------ PromptTests

func TestBasics_EveryTemplateRenders(t *testing.T) {
	assertEqual(t, work.CheckTemplates(), []string{})
}

func TestBasics_DataFenceOutgrowsBackticksInside(t *testing.T) {
	wrapped := prompts.Data("before ```` after", "Thing")
	assertIn(t, "`````text", wrapped)
	assertIn(t, "data, not instructions", wrapped)
}

func TestBasics_SystemPromptNamesTheForbiddenPaths(t *testing.T) {
	text, err := prompts.Render("system", config.PromptsDir, pyjson.New("bot", "b", "repo", "r",
		"title", "night bot", "who", "the night bot"))
	if err != nil {
		t.Fatal(err)
	}
	for _, path := range []string{".github/", ".harness/", ".squishy/", "bot/"} {
		assertIn(t, path, text)
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
	assertIn(t, "bot-state", gh.Branches)
	assertTrue(t, pyjson.Truthy(store.Load().Get("halted")))
	gh.ConflictsToInject = 2
	store.Update(func(s *pyjson.Object) any { state.Item(s, 5).Set("failures", 1); return nil }, "fail")
	loaded := store.Load()
	assertEqual(t, loaded.Obj("items").Obj("5").Get("failures"), 1)
	assertIn(t, "999", loaded.Obj("items")) // the other writer's change survived
	assertTrue(t, pyjson.Truthy(loaded.Get("halted")))
}

func TestBasics_UpdateGivesUpEventually(t *testing.T) {
	gh := fakes.NewFakeGitHub()
	store := basicsStore(gh)
	store.Update(func(s *pyjson.Object) any { s.Set("halted", false); return nil }, "x")
	gh.ConflictsToInject = 99
	assertRaises[*herr.StateConflict](t, func() {
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
		assertRaises[*herr.StateUnreadable](t, func() { store.Load() })
		assertRaises[*herr.StateUnreadable](t, func() {
			store.Update(func(s *pyjson.Object) any { s.Set("halted", false); return nil }, "y")
		})
		assertEqual(t, gh.Files[[2]string{"bot-state", "state.json"}], [2]string{text, "sha-kept"})
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
	assertEqual(t, [2]string{text, sha}, [2]string{big, "abc123"})
	assertTrue(t, strings.HasSuffix(asked[len(asked)-1], "/repos/o/r/git/blobs/abc123"), asked)
}

// ------------------------------------------------------------------ PathTests

func TestBasics_ForbiddenAndReviewPaths(t *testing.T) {
	cfg := testsupport.MakeConfig()
	forbidden, review := cfg.ForbiddenPaths, cfg.ReviewPaths
	for _, path := range []string{".github/workflows/x.yml", ".GitHub/workflows/x.yml", "./bot/harness/gh.py",
		".harness/trust.txt", ".claude/settings.json", ".mcp.json", ".vscode/tasks.json", ".github"} {
		assertTrue(t, git.Matches(path, forbidden), path)
	}
	for _, path := range []string{"package.json", "apps/web/package.json", "pnpm-lock.yaml",
		"Cargo.toml", "crates/engine/Cargo.toml", "Cargo.lock", "rust-toolchain.toml",
		"rustfmt.toml", "crates/engine/clippy.toml", "crates/cards/build.rs",
		".cargo/config.toml", "vitest.config.ts", "apps/web/tsconfig.json",
		"eslint.config.js", "scripts/build-wasm.sh", "wrangler.jsonc", "apps/edge/src/index.ts",
		"apps/web/public/_headers",
		"crates/server/Dockerfile", "crates/server/migrations/0005_x.sql",
		"e2e/cypress.config.ts"} {
		assertTrue(t, git.Matches(path, review), path)
	}
	for _, path := range []string{"apps/web/src/game/Game.tsx", "crates/cards/src/scripts/core/c002_bigot.rs",
		"SPEC.md", "bots/x", "docs/scripts.md", "crates/engine/tests/rules/combat.rs"} {
		assertFalse(t, git.Matches(path, forbidden), path)
		assertFalse(t, git.Matches(path, review), path)
	}
}

func TestBasics_AnExceptionTakesAPathBackOut(t *testing.T) {
	assertFalse(t, git.Matches(".claude/skills/x/SKILL.md", []string{".claude/", "!.claude/skills/"}))
	assertTrue(t, git.Matches(".claude/settings.json", []string{".claude/", "!.claude/skills/"}))
	assertTrue(t, git.Matches(".claude/skillset.json", []string{".claude/", "!.claude/skills/"}))
	assertFalse(t, git.Matches("x.md", []string{"!x.md"}))
	// Squishy may change the skills (the weekly hygiene issue, #839); the night bot may not, and
	// neither may change the rest of .claude/.
	for _, c := range []struct {
		home            string
		skillsForbidden bool
	}{{".harness", true}, {".squishy", false}} {
		raw := loadsObject(t, readText(t, filepath.Join(config.RepoRoot, c.home, "config.json")))
		forbidden := pyjson.Strings(raw.Get("forbidden_paths"))
		assertEqual(t, git.Matches(".claude/skills/maxi-tail/SKILL.md", forbidden), c.skillsForbidden, c.home)
		for _, path := range []string{".claude/settings.json", ".claude/settings.local.json", ".claude/hooks/x.sh"} {
			assertTrue(t, git.Matches(path, forbidden), c.home, " ", path)
		}
	}
}
