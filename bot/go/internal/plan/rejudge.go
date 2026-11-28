package plan

// Suggestions the judge never answered (#5104). A survey's judge (`work.Worker.judge`) can give no
// word on a suggestion: its call is refused at the subscription's cap, times out, fails, or answers
// without a verdict for it. Delivery then opens the suggestion with no `**Judge:**` line, and until
// now it waited for a person however clear it was. So the next survey judges such suggestions again
// before its own work (`Unjudged`, the plan's `rejudge`), usually on another subscription, and
// delivery puts the judge's word on each as a comment (`deliver.Deliverer.applyRejudged`). One the
// judge still cannot answer after `RejudgeTries` surveys is left to a person, with a comment that says
// so.

import (
	"sort"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/identity"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// RejudgeMax is how many unanswered suggestions one survey judges again before its own work.
var RejudgeMax = 3

// RejudgeTries is how many surveys try to judge a suggestion again before it is left to a person.
var RejudgeTries = 3

// JudgeMarker marks the bot's comment that carries the judge's word on a suggestion opened without
// it, so no later survey judges that suggestion again.
const JudgeMarker = "<!-- jackioh-judge -->"

// JudgeLine is how a suggestion's footer or comment gives the judge's word.
const JudgeLine = "**Judge:**"

// footerStart is where delivery's footer begins in a suggestion's body.
const footerStart = "\n\n---\nSuggested by @"

// Unjudged is the open suggestions the judge never answered, oldest first and at most `RejudgeMax`,
// each as {number, title, body} with its body cut before the bot's footer. Skipped: one whose footer
// or a bot comment already gives the judge's word, and one a person took up (a comment or a 👎,
// `heldIn`), which waits for their vote. None while auto-approval is off.
func Unjudged(ctx *hctx.Context) []*pyjson.Object {
	if !ctx.Cfg.AutoApprove.Enabled || !config.Identity.Suggestions {
		return nil
	}
	issues := []*pyjson.Object{}
	for _, issue := range ctx.GH.ListIssues(config.LabelSuggestion, "open", 300, "") {
		if !issue.Has("pull_request") && !strings.Contains(issue.Str("body"), JudgeLine) {
			issues = append(issues, issue)
		}
	}
	sort.Slice(issues, func(i, j int) bool { return issues[i].Int("number") < issues[j].Int("number") })
	out := []*pyjson.Object{}
	for _, issue := range issues {
		if len(out) >= RejudgeMax {
			break
		}
		number := issue.Int("number")
		comments := ctx.GH.ListComments(number, 100)
		if judgedIn(comments) || heldIn(ctx, issue, comments) != "" {
			continue
		}
		body := issue.Str("body")
		if at := strings.Index(body, footerStart); at >= 0 {
			body = body[:at]
		}
		out = append(out, pyjson.New("number", number, "title", py.OrStr(issue.Get("title"), ""), "body", body))
	}
	return out
}

// judgedIn is whether one of `comments` is the bot's comment with the judge's word.
func judgedIn(comments []*pyjson.Object) bool {
	for _, c := range comments {
		if identity.Carries(c.Str("body"), JudgeMarker) {
			return true
		}
	}
	return false
}
