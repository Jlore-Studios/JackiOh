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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// MAX_FINDINGS = 20
var MaxFindings int = 20

// `(parsed JSON or None, the text after the header)`.

func Header(text string, tag string) (any, string) {
	panic("unported: verdicts.header")
}

// RATINGS = ("easy", "medium", "hard")
var Ratings = []string{"easy", "medium", "hard"}

// A planner's rating, `{"difficulty", "why"}`, from the `<!-- bot: ... -->` line its answer
// starts with (#317 part 8), and the plan without it. Only a line at the very start counts: a
// plan may quote such a line further down.

func Rating(text string) (map[string]string, string) {
	panic("unported: verdicts.rating")
}

// BuildReport is verdicts.BuildReport.

type BuildReport struct {
	Status        string
	Title         string
	Question      string
	Body          string
	NextModel     string // default: ""
	HumanRequired bool   // default: False
}

// build_report is verdicts.build_report.

func BuildReport_(text string) *BuildReport {
	panic("unported: verdicts.build_report")
}

// Finding is verdicts.Finding.

type Finding struct {
	Severity string
	Where    string
	Claim    string
	Evidence string
}

func (x *Finding) ToDict() *pyjson.Object {
	panic("unported: verdicts.Finding.to_dict")
}

func (x *Finding) Markdown() string {
	panic("unported: verdicts.Finding.markdown")
}

// Review is verdicts.Review.

type Review struct {
	Readable    bool
	Verdict     string
	Findings    []*Finding // default: field(default_factory=list)
	Body        string     // default: ""
	ReviewedSHA string     // default: ""
	Error       string     // default: ""
}

func (x *Review) Blocking() []*Finding {
	panic("unported: verdicts.Review.blocking")
}

func (x *Review) Notes() []*Finding {
	panic("unported: verdicts.Review.notes")
}

// No blocking finding in a readable review. A `changes` verdict that names nothing
// blocking has nothing for a builder to fix, so it counts as approval too.

func (x *Review) Approved() bool {
	panic("unported: verdicts.Review.approved")
}

func (x *Review) ToDict() *pyjson.Object {
	panic("unported: verdicts.Review.to_dict")
}

// Why there was no verdict, for a reason or a finding.

func (x *Review) WhyUnreadable() string {
	panic("unported: verdicts.Review.why_unreadable")
}

// review is verdicts.review.

func Review_(text string) *Review {
	panic("unported: verdicts.review")
}

// Suggestion is verdicts.Suggestion.

type Suggestion struct {
	Title string
	Body  string
	Kind  string // default: ""
}

// suggestions is verdicts.suggestions.

func Suggestions(text string, limit int) []*Suggestion {
	panic("unported: verdicts.suggestions")
}
