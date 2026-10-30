// Package verdicts is bot/harness/verdicts.py.
//
// Reading the one-line JSON header each model call opens its final message with.
//
// `<!-- bot: {...} -->` from a builder, `<!-- review: {...} -->` from the reviewer and
// `<!-- suggestions: [...] -->` from the suggestion survey. The header is searched for anywhere in
// the message, so a model that wrote a sentence first is still read; the text after it is the
// report a person sees.
package verdicts

import (
	"fmt"
	"regexp"
	"slices"
	"strings"
	"sync"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MaxFindings = 20
var MaxFindings int = 20

var (
	headersMu sync.Mutex
	headers   = map[string]*regexp.Regexp{}
)

// headerPattern is `<!--\s*TAG\s*:\s*`, compiled once a tag.
func headerPattern(tag string) *regexp.Regexp {
	headersMu.Lock()
	defer headersMu.Unlock()
	re, ok := headers[tag]
	if !ok {
		ws := `[` + py.SpaceClass + `]*`
		re = regexp.MustCompile(`<!--` + ws + regexp.QuoteMeta(tag) + ws + `:` + ws)
		headers[tag] = re
	}
	return re
}

// Header is `(parsed JSON or None, the text after the header)`.
func Header(text string, tag string) (any, string) {
	source := text
	match := headerPattern(tag).FindStringIndex(source)
	if match == nil {
		return nil, py.Strip(source)
	}
	tail := source[match[1]:]
	value, used, ok := rawDecode(tail)
	if !ok {
		if end := strings.Index(tail, "-->"); end >= 0 {
			return nil, py.Strip(tail[end+3:])
		}
		return nil, py.Strip(tail)
	}
	rest := tail[used:]
	if close := strings.Index(rest, "-->"); close >= 0 {
		rest = rest[close+3:]
	}
	return value, py.Strip(rest)
}

// Ratings are the difficulties a planner may rate an item (`config.DIFFICULTIES`).
var Ratings = []string{"easy", "medium", "hard"}

// Rating is a planner's rating, `{"difficulty", "why"}`, from the `<!-- bot: ... -->` line its
// answer starts with (#317 part 8), and the plan without it. Only a line at the very start counts:
// a plan may quote such a line further down. A nil map is None.
func Rating(text string) (map[string]string, string) {
	source := py.Strip(text)
	if !strings.HasPrefix(source, "<!--") {
		return nil, source
	}
	value, rest := Header(source, "bot")
	found, ok := value.(*pyjson.Object)
	if !ok {
		return nil, source
	}
	difficulty := py.Lower(py.Strip(py.OrStr(found.Get("difficulty"), "")))
	if !slices.Contains(Ratings, difficulty) {
		return nil, rest
	}
	return map[string]string{"difficulty": difficulty,
		"why": py.SliceTo(strings.Join(py.Split(py.OrStr(found.Get("why"), "")), " "), 300)}, rest
}

// BuildReport is verdicts.BuildReport. Status is "done", "blocked" or "unknown".
type BuildReport struct {
	Status   string
	Title    string
	Question string
	Body     string
	// NextModel is the model the builder asks this run's next pass to run on (`"next_model"`),
	// lowercased: a lane with two models switches to it (`work.Worker._switch`). "" stays.
	NextModel string // default: ""
	// HumanRequired: a blocked builder says only a person can do it (`"human_required": true`,
	// #793): a change to a path the bot may not touch, a secret, a dashboard or an account
	// (`human.py`).
	HumanRequired bool // default: False
}

// BuildReport_ is verdicts.build_report.
func BuildReport_(text string) *BuildReport {
	value, rest := Header(text, "bot")
	found, ok := value.(*pyjson.Object)
	if !ok {
		return &BuildReport{Status: "unknown", Body: rest}
	}
	status := py.Lower(py.Str(found.GetOr("status", "")))
	if status != "done" && status != "blocked" {
		status = "unknown"
	}
	title := py.SliceTo(strings.Join(py.Split(py.Str(found.GetOr("title", ""))), " "), 200)
	question := py.SliceTo(py.Strip(py.Str(found.GetOr("question", ""))), 4000)
	nextModel := py.SliceTo(py.Lower(strings.Trim(py.Strip(py.OrStr(found.Get("next_model"), "")), "`")), 40)
	human := status == "blocked" && found.Get("human_required") == true
	return &BuildReport{status, title, question, rest, nextModel, human}
}

// Finding is verdicts.Finding.
type Finding struct {
	Severity string
	Where    string
	Claim    string
	Evidence string
}

func (x *Finding) ToDict() *pyjson.Object {
	return pyjson.New("severity", x.Severity, "where", x.Where, "claim", x.Claim, "evidence", x.Evidence)
}

func (x *Finding) Markdown() string {
	evidence := ""
	if x.Evidence != "" {
		evidence = " Evidence: " + x.Evidence
	}
	return fmt.Sprintf("- **%s** `%s`: %s%s", x.Severity, x.Where, x.Claim, evidence)
}

// Review is verdicts.Review. Verdict is "approve", "changes" or "unreadable".
type Review struct {
	Readable    bool
	Verdict     string
	Findings    []*Finding // default: field(default_factory=list)
	Body        string     // default: ""
	ReviewedSHA string     // default: ""
	// Error: the reviewer's call failed (`RunResult.error`), so there was no answer to read.
	Error string // default: ""
}

func (x *Review) Blocking() []*Finding {
	found := []*Finding{}
	for _, f := range x.Findings {
		if f.Severity == "blocking" {
			found = append(found, f)
		}
	}
	return found
}

func (x *Review) Notes() []*Finding {
	found := []*Finding{}
	for _, f := range x.Findings {
		if f.Severity != "blocking" {
			found = append(found, f)
		}
	}
	return found
}

// Approved is no blocking finding in a readable review. A `changes` verdict that names nothing
// blocking has nothing for a builder to fix, so it counts as approval too.
func (x *Review) Approved() bool {
	return x.Readable && len(x.Blocking()) == 0
}

func (x *Review) ToDict() *pyjson.Object {
	findings := []any{}
	for _, f := range x.Findings {
		findings = append(findings, f.ToDict())
	}
	found := pyjson.New("verdict", x.Verdict, "readable", x.Readable, "findings", findings,
		"body", py.SliceTo(x.Body, 20000))
	if x.Error != "" {
		found.Set("error", py.SliceTo(x.Error, 2000))
	}
	return found
}

// WhyUnreadable is why there was no verdict, for a reason or a finding.
func (x *Review) WhyUnreadable() string {
	if x.Error != "" {
		return "the reviewer's call failed: " + py.SliceTo(x.Error, 500)
	}
	return "the reviewer's answer could not be read twice in a row"
}

// Review_ is verdicts.review.
func Review_(text string) *Review {
	value, rest := Header(text, "review")
	found, ok := value.(*pyjson.Object)
	if !ok {
		return &Review{Readable: false, Verdict: "unreadable", Findings: []*Finding{}, Body: rest}
	}
	verdict := py.Lower(py.Str(found.GetOr("verdict", "")))
	findings := []*Finding{}
	if raw, ok := orNil(found.Get("findings")).([]any); ok {
		if len(raw) > MaxFindings {
			raw = raw[:MaxFindings]
		}
		for _, entry := range pyjson.Objs(raw) {
			severity := py.Lower(py.Str(entry.GetOr("severity", "blocking")))
			if severity != "note" {
				severity = "blocking"
			}
			findings = append(findings, &Finding{
				severity,
				py.SliceTo(py.Str(entry.GetOr("where", "?")), 300),
				py.SliceTo(py.Strip(py.Str(entry.GetOr("claim", ""))), 2000),
				py.SliceTo(py.Strip(py.Str(entry.GetOr("evidence", ""))), 2000),
			})
		}
	}
	if verdict != "approve" && verdict != "changes" {
		return &Review{Readable: false, Verdict: "unreadable", Findings: findings, Body: rest}
	}
	return &Review{Readable: true, Verdict: verdict, Findings: findings, Body: rest}
}

// Suggestion is verdicts.Suggestion.
type Suggestion struct {
	Title string
	Body  string
	// Kind is its class id (`prompts/suggest.md`, #1409), or "" when the survey named none.
	Kind string // default: ""
}

// Suggestions is verdicts.suggestions.
func Suggestions(text string, limit int) []*Suggestion {
	value, _ := Header(text, "suggestions")
	entries, ok := value.([]any)
	if !ok {
		return []*Suggestion{}
	}
	found := []*Suggestion{}
	for _, e := range entries {
		entry, ok := e.(*pyjson.Object)
		if !ok {
			continue
		}
		title := py.SliceTo(strings.Join(py.Split(py.Str(entry.GetOr("title", ""))), " "), 200)
		body := py.Strip(py.Str(entry.GetOr("body", "")))
		kind := ""
		if k, ok := entry.Get("class").(string); ok {
			kind = py.Strip(k)
		}
		if title != "" && body != "" {
			found = append(found, &Suggestion{title, body, kind})
		}
		if len(found) >= limit {
			break
		}
	}
	return found
}

// orNil is `v or []`'s falsy case: a falsy value reads as an empty list.
func orNil(v any) any {
	if !pyjson.Truthy(v) {
		return []any{}
	}
	return v
}
