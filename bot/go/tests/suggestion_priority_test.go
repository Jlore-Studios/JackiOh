// #1409: a suggestion opens with the priority label its class maps to in `.harness/config.json`
// (`suggestions.priorities`), and approving it keeps that label.
//
// bot/tests/test_suggestion_priority.py, ported (#467).
package tests

import (
	"regexp"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/prompts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/runner"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/verdicts"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/work"
)

// suggestionPrioritySurvey is survey(*entries): a survey that answers these suggestions.
func suggestionPrioritySurvey(entries ...*pyjson.Object) *runner.FakeRunner {
	parts := []string{}
	for _, e := range entries {
		parts = append(parts, pyjson.Dumps(e))
	}
	text := "<!-- suggestions: [" + strings.Join(parts, ",") + "] -->"
	return workFake(map[string]any{"suggest": workReviewer(text)})
}

// suggestionPriorityIdea is idea(title, kind=None): a suggestion, of class `kind` when given.
func suggestionPriorityIdea(title string, kind ...any) *pyjson.Object {
	entry := pyjson.New("title", title, "body", "## Why\nx")
	if len(kind) > 0 && kind[0] != nil {
		entry.Set("class", kind[0])
	}
	return entry
}

// suggestionPriorityOpened is opened(h): the suggestions the bot opened, by title.
func suggestionPriorityOpened(h *flowHarness) map[string]*pyjson.Object {
	made := map[string]*pyjson.Object{}
	for _, thread := range h.gh.Threads.Values() {
		if pyjson.Equal(thread.Get("user"), fakes.Bot) && flowThreadLabels(thread)[config.LabelSuggestion] {
			made[thread.Str("title")] = thread
		}
	}
	return made
}

// suggestionPriorityWith is with_priorities(priorities): config.json's `suggestions` with this
// table.
func suggestionPriorityWith(priorities any) *pyjson.Object {
	return testsupport.RawConfig().Obj("suggestions").Copy().Set("priorities", priorities)
}

// ---------------------------------------------------------------------------- DeliveryTests

func TestSuggestionPriority_EachClassOpensWithItsPriority(t *testing.T) {
	h := newFlowHarness(t)
	testsupport.IdleFor(h.ctx)
	r := suggestionPrioritySurvey(suggestionPriorityIdea("Fuzz", "fuzz"), suggestionPriorityIdea("Security", "security"),
		suggestionPriorityIdea("Data", "data"), suggestionPriorityIdea("Feature", "feature"),
		suggestionPriorityIdea("Unknown", "nonsense"), suggestionPriorityIdea("None"), suggestionPriorityIdea("Odd", 7))
	h.night(r)
	made := suggestionPriorityOpened(h)
	labels := map[string]map[string]bool{}
	for title, thread := range made {
		labels[title] = flowThreadLabels(thread)
	}
	assertEqual(t, labels, map[string]map[string]bool{
		"Fuzz":     setOf(config.LabelSuggestion, "priority:low"),
		"Security": setOf(config.LabelSuggestion, "priority:medium"),
		"Data":     setOf(config.LabelSuggestion, "priority:high"),
		"Feature":  setOf(config.LabelSuggestion),
		"Unknown":  setOf(config.LabelSuggestion),
		"None":     setOf(config.LabelSuggestion),
		"Odd":      setOf(config.LabelSuggestion),
	})
	if made["Fuzz"] == nil || made["Feature"] == nil {
		t.Fatal("KeyError: the survey opened no Fuzz or Feature")
	}
	assertIn(t, "Class: `fuzz` → `priority:low`; relabel to change it.", made["Fuzz"].Str("body"))
	assertNotIn(t, "Class:", made["Feature"].Str("body"))
	// The survey was told every class and the priority it sets, highest first.
	prompt := r.Calls[0].Prompt
	assertIn(t, "- `data`: `priority:high`\n- `security`: `priority:medium`", prompt)
	// In pickup order: no priority comes before low, so `feature` wins a tie with `fuzz`.
	assertIn(t, "- `polish`: `priority:medium`\n- `feature`: no priority (the owner's call)\n"+
		"- `fuzz`: `priority:low`", prompt)
}

func TestSuggestionPriority_TheConfigAloneSetsThePriority(t *testing.T) {
	h := newFlowHarness(t, flowHarnessOpts{Overrides: pyjson.New("suggestions",
		suggestionPriorityWith(pyjson.New("fuzz", "high")))})
	testsupport.IdleFor(h.ctx)
	h.night(suggestionPrioritySurvey(suggestionPriorityIdea("Fuzz", "fuzz"), suggestionPriorityIdea("Data", "data")))
	made := suggestionPriorityOpened(h)
	if made["Fuzz"] == nil || made["Data"] == nil {
		t.Fatal("KeyError: the survey opened no Fuzz or Data")
	}
	assertEqual(t, flowThreadLabels(made["Fuzz"]), setOf(config.LabelSuggestion, "priority:high"))
	assertEqual(t, flowThreadLabels(made["Data"]), setOf(config.LabelSuggestion))
}

func TestSuggestionPriority_ApprovingKeepsThePriority(t *testing.T) {
	h := newFlowHarness(t)
	h.gh.AddIssue(20, fakes.Issue{Labels: flowLabels(config.LabelSuggestion, "priority:low")})
	flowAsk(h, 20, "approve", 41)
	assertEqual(t, h.gh.LabelNames(20), setOf(config.LabelBuild, "priority:low"))
}

// ---------------------------------------------------------------------------- ConfigTests

func TestSuggestionPriority_TheCommittedTable(t *testing.T) {
	cfg := config.Load("", map[string]string{})
	assertEqual(t, cfg.SuggestionPriorities, map[string]string{
		"fuzz": "low", "checks": "low", "telemetry": "low", "refactor": "low", "ai": "low",
		"fairness": "low", "security": "medium", "polish": "medium", "data": "high"})
}

func TestSuggestionPriority_BadTablesFailToLoad(t *testing.T) {
	for _, c := range []struct {
		priorities any
		message    string
	}{{pyjson.New("fuzz", "urgent"), "fuzz: 'urgent' is not one of"},
		{pyjson.New("fuzz", nil), "fuzz: None is not one of"},
		{pyjson.New("bugs", "low"), "unknown classes bugs"},
		{[]any{"fuzz"}, "expected an object"}} {
		raw := testsupport.RawConfig()
		raw.Set("suggestions", suggestionPriorityWith(c.priorities))
		err, ok := herr.Catch[*herr.ConfigError](func() { config.Parse(raw, config.RepoRoot, map[string]string{}, nil) })
		if !ok {
			t.Errorf("ConfigError not raised for %s", show(c.priorities))
			continue
		}
		assertTrue(t, regexp.MustCompile(c.message).MatchString(err.Error()), c.message, err.Error())
	}
}

func TestSuggestionPriority_NoTableSetsNoPriority(t *testing.T) {
	raw := testsupport.RawConfig()
	if _, ok := raw.Obj("suggestions").Pop("priorities"); !ok {
		t.Fatal("KeyError: 'priorities'")
	}
	cfg := config.Parse(raw, config.RepoRoot, map[string]string{}, nil)
	assertEqual(t, len(cfg.SuggestionPriorities), 0)
	assertIn(t, "- `data`: no priority", work.ClassPriorities(cfg.SuggestionPriorities))
}

// ---------------------------------------------------------------------------- PromptTests

func TestSuggestionPriority_TheSurveyListsEveryClass(t *testing.T) {
	template, err := prompts.Load("suggest", config.PromptsDir)
	if err != nil {
		t.Fatal(err)
	}
	text := template.Template
	assertTrue(t, strings.HasPrefix(text, "<!-- version: 4 -->"), text[:min(len(text), 40)])
	for _, kind := range config.SuggestionClasses {
		assertIn(t, "- `"+kind+"`: ", text)
	}
	names, err := prompts.Placeholders("suggest", config.PromptsDir)
	if err != nil {
		t.Fatal(err)
	}
	assertTrue(t, names["priorities"], names)
}

func TestSuggestionPriority_TheClassIsRead(t *testing.T) {
	text := `<!-- suggestions: [{"title": "A", "body": "B", "class": " fuzz "}, ` +
		`{"title": "C", "body": "D", "class": ["x"]}, {"title": "E", "body": "F"}] -->`
	kinds := []string{}
	for _, s := range verdicts.Suggestions(text, 4) {
		kinds = append(kinds, s.Kind)
	}
	assertEqual(t, kinds, []string{"fuzz", "", ""})
}
