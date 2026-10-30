// Package quiet is bot/harness/quiet.py.
//
// Start only when the Claude subscription is quiet: nobody else is spending it.
//
// The bot shares one subscription with the person who owns it and with a partner bot
// (bright-bots-harness). Before a run claims work, it reads the subscription's usage with the
// smallest model call there is, waits `interval_minutes`, and reads it again. Usage that did not
// rise means nobody else is working, so the run goes ahead. Usage that rose while the partner was
// in one of its spending steps is the partner's, and does not hold the bot back. Any other rise is
// a person or another agent, so it waits and looks again, up to `max_wait_minutes`, then gives up
// until the next run. A forced run skips all of this. The partner implements the same rule.
package quiet

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"time"
)

// EPSILON = 0.001
var Epsilon float64 = 0.001

// Sample is quiet.Sample.

type Sample struct {
	At          time.Time
	FiveHour    *float64
	SevenDay    *float64
	FiveResets  string
	SevenResets string
	Status      string
}

func SampleOf(usage *pyjson.Object, at time.Time) *Sample {
	panic("unported: quiet.Sample.of")
}

func (x *Sample) ToDict() *pyjson.Object {
	panic("unported: quiet.Sample.to_dict")
}

// `quiet`, `rose`, `inconclusive` (a window turned over between the two) or `refused`.

func Compare(a *Sample, b *Sample) string {
	panic("unported: quiet.compare")
}

// Which partner step spent the subscription at some point in [t1, t2], or None.
//
// Reads each partner's recent runs and their jobs' steps. An error reading a partner counts as
// not spending, so a rise is never excused on a guess.

func PartnerSpending(partners []*config.Partner, clientFor func(string) any, t1 time.Time, t2 time.Time) string {
	panic("unported: quiet.partner_spending")
}

// Verdict is quiet.Verdict.

type Verdict struct {
	Quiet     bool
	Reason    string
	Samples   []*pyjson.Object // default: field(default_factory=list)
	ExcusedBy string           // default: None
}

func (x *Verdict) ToDict() *pyjson.Object {
	panic("unported: quiet.Verdict.to_dict")
}

// Sample, wait an interval, sample again, until the subscription is quiet or the wait is up.

func WaitForQuiet(settings *config.Quiet, ping func() *pyjson.Object, partner func(time.Time, time.Time) string, now func() time.Time, sleep func(float64)) *Verdict {
	panic("unported: quiet.wait_for_quiet")
}

// Reads a partner's public Actions runs: with the job's token first, then without one,
// since a job's own token may not reach another repository.

type PartnerReader struct {
	Clients any // [make(repo, token)] if token else []
}

// NewPartnerReader is PartnerReader(...).

func NewPartnerReader(repo string, token string, factory func(...any) any /* = None */) *PartnerReader {
	panic("unported: quiet.PartnerReader.__init__")
}

func (x *PartnerReader) call(name string, kwargs *pyjson.Object, args ...any) any {
	panic("unported: quiet.PartnerReader._call")
}

func (x *PartnerReader) ListRuns(workflow string, limit int /* = 10 */) []*pyjson.Object {
	panic("unported: quiet.PartnerReader.list_runs")
}

func (x *PartnerReader) ListJobs(runID any) []*pyjson.Object {
	panic("unported: quiet.PartnerReader.list_jobs")
}
