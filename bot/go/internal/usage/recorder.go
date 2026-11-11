package usage

import (
	"errors"
	"fmt"
	"os"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// FlushEvery is how often the loop writes its rows to its clone of the branch, derives what
// closed, draws the usage issue (`Draw`) and pushes: every 30 minutes, the statistics count's
// cadence until #3675 moved that count to every 10 on a goroutine of its own, so a rewrite stays
// as fast as it was.
var FlushEvery = 30 * time.Minute

// PushAttempts is how many times one flush pushes, writing its rows again on the branch's new tip
// after each push the branch's moving refused.
var PushAttempts = 3

// NewestLookback is how far back the loop reads the history, once, for each subscription's newest
// reading, which the state file's reading must differ from to be recorded.
var NewestLookback = 7 * 24 * time.Hour

// Remote is the repository the loop clones the usage branch from and pushes it to; a variable so
// the tests can point it at a repository on disk.
var Remote = func(cfg *config.Config) string { return cfg.ServerURL + "/" + cfg.Repo + ".git" }

// Scratch is where the loop keeps its clones: the runner's RUNNER_TEMP in a workflow, else the
// system's temporary directory; a variable so the tests can keep them in theirs.
var Scratch = func() string {
	if dir := config.GithubEnv()["RUNNER_TEMP"]; dir != "" {
		return dir
	}
	return os.TempDir()
}

// Token is what the loop's git hands GitHub: the job's own token (`contents: write` in
// bot-status.yml), whose pushes start no workflow, else the bot's.
func Token(cfg *config.Config) string {
	if cfg.ActionsToken != "" {
		return cfg.ActionsToken
	}
	return cfg.BotToken
}

// Recorder is the status loop's usage history, for the loop's lifetime (`harness dashboard
// --every …`): each tick it gathers rows in memory (`Tick`); at the first tick and every
// `FlushEvery` after it writes them to a local clone of the branch, commits and pushes (`Flush`),
// and once more when the loop ends (`Close`). A failure is logged, and the loop goes on: rows not
// pushed stay in memory, and the spool's stay in the state file, for the next flush.
type Recorder struct {
	// Log takes each line it has to say (the loop's output).
	Log func(string)
	// Since is the first day a backfill reads (`BackfillSince` unless a person asked otherwise),
	// and Rebackfill makes a flush read the journal again even with a marker (`harness usage
	// backfill`).
	Since      time.Time
	Rebackfill bool

	branch  *Branch
	history *History
	// pending is every row not pushed yet (written to the clone or not), in the order it came.
	pending []Row
	// marker is the backfill's marker not pushed yet.
	marker *pyjson.Object
	// taken is the spool's lines among `pending`, by kind name, which a push takes out of it.
	taken    map[string][]string
	takenSet map[string]bool
	lastTick map[string]*Tick
	lastPool *PoolTick
	// newest is each subscription's newest reading the history, the spool or this loop holds.
	newest  map[string]*Reading
	flushed time.Time
	// tried is when the recorder last tried to clone the branch, so one tick tries once.
	tried time.Time
	// Problem is the last failure this recorder logged, "" for none since it was made.
	Problem string
	// derived: a flush of this loop derived the windows, looking `DeriveLookback` back; the later
	// ones look `DeriveRecent` back.
	derived bool
}

// NewRecorder is a recorder that says what it does through `log`.
func NewRecorder(log func(string)) *Recorder {
	return &Recorder{Log: log, Since: BackfillSince, taken: map[string][]string{}, takenSet: map[string]bool{},
		lastTick: map[string]*Tick{}, newest: map[string]*Reading{}}
}

func (x *Recorder) log(line string) {
	if x.Log != nil {
		x.Log(redact.Redact("usage: "+line, nil))
	}
}

// warn logs a failure as a workflow warning, and keeps it as the recorder's last (`Problem`).
func (x *Recorder) warn(line string) {
	x.Problem = line
	x.log("::warning::" + line)
}

// Root is the clone's root, "" before the first flush cloned it.
func (x *Recorder) Root() string {
	if x.branch == nil {
		return ""
	}
	return x.branch.Dir
}

// Tick records one status tick from what it read for the status issue (`dashboard.Looked`): the
// spool's rows, the state file's newest reading of each subscription that no run handed on, a tick
// row per subscription that changed (or has had none for `TickEvery`) and the pool's; then, when
// due, a flush. It reads nothing from GitHub but at a flush, and never raises.
func (x *Recorder) Tick(ctx *hctx.Context, facts TickFacts) {
	if !ctx.Cfg.RecordsUsage() {
		return
	}
	if err := herr.Try(func() { x.tick(ctx, facts) }); err != nil {
		x.warn("this tick was not recorded: " + herr.Str(err))
	}
}

func (x *Recorder) tick(ctx *hctx.Context, facts TickFacts) {
	now := utc(ctx.Now())
	due := x.flushed.IsZero() || now.Sub(x.flushed) >= FlushEvery
	if due && x.branch == nil {
		x.open(ctx) // the first: what the history holds tells which state readings are new
	}
	x.takeSpool(facts.State)
	for _, reading := range StateReadings(ctx, facts.State) {
		if newest := x.newest[reading.Subscription]; newest != nil &&
			(!reading.ObservedAt.After(newest.ObservedAt) || reading.sameUsage(newest)) {
			continue // the one a delivery noted, which its run's readings hold
		}
		x.add(reading)
	}
	ticks, pool := TickRows(ctx, facts)
	for _, t := range ticks {
		last := x.lastTick[t.Subscription]
		if t.changed(last) || now.Sub(last.At) >= TickEvery {
			x.add(t)
			x.lastTick[t.Subscription] = t
		}
	}
	if pool.changed(x.lastPool) || now.Sub(x.lastPool.At) >= TickEvery {
		x.add(pool)
		x.lastPool = pool
	}
	if due {
		x.flush(ctx)
	}
}

// add keeps `row` for the next push, and each subscription's newest reading.
func (x *Recorder) add(row Row) {
	x.pending = append(x.pending, row)
	if reading, ok := row.(*Reading); ok {
		if newest := x.newest[reading.Subscription]; newest == nil || reading.ObservedAt.After(newest.ObservedAt) {
			x.newest[reading.Subscription] = reading
		}
	}
}

// takeSpool keeps the spool's rows the recorder has not taken yet.
func (x *Recorder) takeSpool(state *pyjson.Object) {
	for _, s := range SpoolOf(state) {
		if x.takenSet[s.Line] {
			continue
		}
		x.takenSet[s.Line] = true
		x.taken[s.Kind.Name] = append(x.taken[s.Kind.Name], s.Line)
		x.add(s.Row)
	}
}

// open clones the usage branch (or starts it), and reads each subscription's newest reading from
// it. A failure is logged; the next flush tries again.
func (x *Recorder) open(ctx *hctx.Context) {
	cfg := ctx.Cfg
	x.tried = utc(ctx.Now())
	b, err := OpenBranch(Scratch(), Remote(cfg), config.Identity.UsageBranch, Token(cfg),
		git.IdentityBot(cfg.BotLogin, cfg.BotUserID))
	if err != nil {
		x.warn(fmt.Sprintf("could not clone `%s`: %s", config.Identity.UsageBranch, herr.Str(err)))
		return
	}
	x.branch, x.history = b, NewHistory(b.Dir)
	if b.New {
		x.log(fmt.Sprintf("`%s` is not there yet: its first push makes it", b.Name))
	}
	readings, err := ReadReadings(b.Dir, ctx.Now().Add(-NewestLookback), time.Time{})
	if err != nil {
		x.warn("could not read the readings: " + err.Error())
	}
	for _, r := range readings {
		if newest := x.newest[r.Subscription]; newest == nil || r.ObservedAt.After(newest.ObservedAt) {
			x.newest[r.Subscription] = r
		}
	}
}

// Flush writes every row not pushed yet to the clone (each once), backfills a branch that was
// never backfilled, derives the sessions and weeks that closed, draws the usage issue (`Draw`), commits
// and pushes; a push the branch's moving refused is tried again on its new tip, up to
// `PushAttempts`. Once pushed, the spool's rows are taken out of the state file. Never raises.
func (x *Recorder) Flush(ctx *hctx.Context) {
	if !ctx.Cfg.RecordsUsage() {
		return
	}
	if err := herr.Try(func() { x.flush(ctx) }); err != nil {
		x.warn("the flush failed: " + herr.Str(err))
	}
}

func (x *Recorder) flush(ctx *hctx.Context) {
	x.flushed = utc(ctx.Now())
	if x.branch == nil && !x.tried.Equal(x.flushed) { // this tick has not tried already
		x.open(ctx)
	}
	if x.branch == nil {
		return // the rows wait, in memory and in the spool, for the next flush
	}
	for attempt := 1; ; attempt++ {
		written, err := x.write(ctx)
		if err != nil {
			x.warn("could not write the rows: " + err.Error())
			return
		}
		if !x.branch.Ahead() {
			x.pushed(ctx, written)
			return
		}
		err = x.branch.Push()
		if err == nil {
			x.pushed(ctx, written)
			x.log(fmt.Sprintf("pushed %d new row(s) to `%s`", written, x.branch.Name))
			return
		}
		if !errors.Is(err, ErrRejected) || attempt >= PushAttempts {
			// The rows stay, in memory and in the clone's commits, for the next flush.
			x.warn(fmt.Sprintf("could not push `%s` (try %d): %s", x.branch.Name, attempt,
				py.SliceTo(err.Error(), 400)))
			return
		}
		if err := x.branch.Sync(); err != nil {
			x.warn("could not fetch the usage branch again: " + herr.Str(err))
			return
		}
		x.history.Forget() // its files moved: what it remembers of their keys is gone
	}
}

// write is one flush's work in the clone: the backfill if it is due, the rows not pushed, the
// sessions and weeks that closed, the usage issue (`Draw`), and a commit. Returns how many rows it
// added to the files.
func (x *Recorder) write(ctx *hctx.Context) (int, error) {
	root := x.branch.Dir
	if x.Rebackfill || (x.marker == nil && !HasMarker(root)) {
		x.backfill(ctx)
	}
	if x.marker != nil { // this loop's backfill, not pushed yet: its marker, over any older one
		if err := WriteMarker(root, x.marker); err != nil {
			return 0, err
		}
	}
	added, err := x.history.Append(x.pending...)
	if err != nil {
		return added, err
	}
	lookback := DeriveRecent
	if !x.derived {
		lookback = DeriveLookback // the loop's first: whatever closed while no loop was going
	}
	sessions, weeks, err := x.history.Derive(utc(ctx.Now()), lookback)
	if err != nil {
		return added, err
	}
	x.derived = true
	if sessions+weeks > 0 {
		x.log(fmt.Sprintf("derived %d session(s) and %d week(s)", sessions, weeks))
	}
	x.draw(ctx)
	_, err = x.branch.Commit(fmt.Sprintf("usage: %d row(s), %d session(s), %d week(s)", added, sessions, weeks))
	return added + sessions + weeks, err
}

// draw draws the usage issue from the clone this flush just wrote (`Draw`): every 30 minutes,
// beside the statistics count, and never on a tick that does not flush. A failure is logged; the
// history is pushed all the same.
func (x *Recorder) draw(ctx *hctx.Context) {
	if Draw == nil {
		return
	}
	if err := herr.Try(func() {
		if note := Draw(ctx, x.branch.Dir); note != "" {
			x.log("the usage issue: " + note)
		}
	}); err != nil {
		x.warn("the usage issue was not drawn: " + herr.Str(err))
	}
}

// backfill reads the journal into rows (once; a person's `harness usage backfill` again), and
// keeps them and its marker for the push. A failure is logged; the next flush tries again.
func (x *Recorder) backfill(ctx *hctx.Context) {
	x.Rebackfill = false
	cfg := ctx.Cfg
	files, sha, err := FetchJournal(Scratch(), Remote(cfg), config.Identity.JournalBranch, Token(cfg))
	if err != nil {
		x.warn(fmt.Sprintf("could not read `%s` for the backfill: %s", config.Identity.JournalBranch,
			herr.Str(err)))
		return
	}
	rows, marker := Backfill(files, cfg.Pool, x.Since, config.Identity.JournalBranch, sha, ctx.Now())
	x.pending = append(rows, x.pending...)
	x.marker = marker
	x.log(fmt.Sprintf("backfilled from `%s` since %s: %s run(s) and %s reading(s) in %d file(s)",
		config.Identity.JournalBranch, x.Since.Format("2006-01-02"), py.Str(marker.Get("runs")),
		py.Str(marker.Get("readings")), len(files)))
}

// pushed forgets what the push took: the rows, the marker, and the spool's lines, which it takes
// out of the state file.
func (x *Recorder) pushed(ctx *hctx.Context, written int) {
	x.pending, x.marker = nil, nil
	if len(x.taken) == 0 {
		return
	}
	taken := x.taken
	if err := herr.Try(func() {
		ctx.Store.Update(func(s *pyjson.Object) any {
			Unspool(s, taken)
			return nil
		}, "usage: spool pushed")
	}); err != nil {
		x.warn("could not take the pushed rows out of the spool: " + herr.Str(err))
		return // they are pushed: the next flush finds them there already, and adds none twice
	}
	x.taken = map[string][]string{}
	for _, lines := range taken {
		for _, l := range lines {
			delete(x.takenSet, l)
		}
	}
}

// Close is the loop's end: a last flush of what it has not pushed, then its clone goes.
func (x *Recorder) Close(ctx *hctx.Context) {
	if !ctx.Cfg.RecordsUsage() {
		return
	}
	if len(x.pending) > 0 || x.marker != nil || len(x.taken) > 0 || (x.branch != nil && x.branch.Ahead()) {
		x.Flush(ctx)
	}
	if x.branch != nil {
		x.branch.Close()
		x.branch, x.history = nil, nil
	}
}
