package dashboard

import (
	"strings"
	"sync"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
)

// Count is a count of the statistics made apart from the rewrite (#3675): the status loop counts
// on a goroutine of its own, so a count's hundreds of calls never hold up the issue, and each
// rewrite draws the block from the newest count. The block is drawn as the count is made, once for
// each number of merged pull requests `Fit` may list (`ShownSteps`), so its windows and its
// "Counted at" are the count's.
type Count struct {
	Blocks map[int]string
}

// CountBox is Counts: the newest count, behind a lock, since the count and the rewrite run on
// goroutines of their own.
type CountBox struct {
	mu     sync.Mutex
	newest *Count
	fresh  bool
}

// Counts is the status loop's newest count (`harness dashboard --stats --every`), which each
// rewrite draws the statistics from instead of counting them itself. The loop sets it for its run
// and puts nil back; with nil, a rewrite counts when they are due, as `harness stats` and
// bot-commands' rewrite do.
var Counts *CountBox

// Put makes `count` the newest.
func (b *CountBox) Put(count *Count) {
	b.mu.Lock()
	defer b.mu.Unlock()
	b.newest, b.fresh = count, true
}

// Newest is the newest count (nil before the first), and whether no rewrite has drawn it yet.
func (b *CountBox) Newest() (*Count, bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	fresh := b.fresh
	b.fresh = false
	return b.newest, fresh
}

// CountNow counts the statistics now, as a rewrite with them due did before #3675: which set's
// cards win most (kept from the issue between its daily reads), every bot comment and pull request,
// and `others`, the other bots' records; then notes in the state file that they were counted. A
// variable so the tests can replace it.
var CountNow = countNow

func countNow(ctx *hctx.Context, others []string, readCards cardsets.Fetch) *Count {
	state := ctx.Store.Load()
	body := ""
	if number := state.Obj("dashboard").Get("issue"); pyjson.Truthy(number) {
		herr.Catch[*herr.GitHubError](func() {
			body = py.OrStr(ctx.GH.GetIssue(py.MustInt(number)).Get("body"), "")
		})
	}
	cards := cardsets.Section(ctx, state, cardsets.Kept(body), readCards)
	facts := stats.Collect(ctx)
	blocks := map[int]string{}
	for _, shown := range ShownSteps {
		blocks[shown] = stats.Block(ctx, facts, shown, others, cards)
	}
	stats.Counted(ctx)
	return &Count{Blocks: blocks}
}

// KeptSections is each other bot's section the status issue holds now (`section`, between
// `SectionStart` and `SectionEnd`): what the loop's rewrites carry until those bots' own processes
// have drawn them afresh, so a loop that starts never drops Squishy's section (#3675). None when
// there is no issue or it cannot be read.
func KeptSections(ctx *hctx.Context) []string {
	number := ctx.Store.Load().Obj("dashboard").Get("issue")
	if !pyjson.Truthy(number) {
		return nil
	}
	body := ""
	if _, failed := herr.Catch[*herr.GitHubError](func() {
		body = py.OrStr(ctx.GH.GetIssue(py.MustInt(number)).Get("body"), "")
	}); failed {
		return nil
	}
	var found []string
	for {
		start := strings.Index(body, SectionStart)
		if start < 0 {
			return found
		}
		end := strings.Index(body[start:], SectionEnd)
		if end < 0 {
			return found
		}
		end += start + len(SectionEnd)
		found = append(found, body[start:end])
		body = body[end:]
	}
}
