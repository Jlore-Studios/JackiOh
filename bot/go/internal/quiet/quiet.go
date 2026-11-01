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
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/gh"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Epsilon is EPSILON: utilization is reported to the hundredth; anything under this is the same
// reading.
var Epsilon float64 = 0.001

// Sample is quiet.Sample.
type Sample struct {
	At       time.Time
	FiveHour *float64
	SevenDay *float64
	// FiveResets and SevenResets are each window's `resets_at` as the reading gave it (a string,
	// or nil for None), compared as Python compares them.
	FiveResets  any
	SevenResets any
	Status      string
}

// SampleOf is Sample.of: a reading as a sample, or nil when it has neither window.
func SampleOf(usage *pyjson.Object, at time.Time) *Sample {
	if usage == nil {
		return nil
	}
	window := func(name string) (*float64, any) {
		reading, ok := usage.Get(name).(*pyjson.Object)
		if !ok || reading == nil {
			return nil, nil
		}
		var share *float64
		switch v := reading.Get("utilization").(type) {
		case int:
			f := float64(v)
			share = &f
		case float64:
			share = &v
		case bool:
			f := 0.0
			if v {
				f = 1
			}
			share = &f
		}
		return share, reading.Get("resets_at")
	}
	five, fiveResets := window("five_hour")
	seven, sevenResets := window("seven_day")
	if five == nil && seven == nil {
		return nil
	}
	return &Sample{At: at, FiveHour: five, SevenDay: seven, FiveResets: fiveResets,
		SevenResets: sevenResets, Status: py.Str(usage.GetOr("status", ""))}
}

// ToDict is Sample.to_dict.
func (x *Sample) ToDict() *pyjson.Object {
	return pyjson.New("at", clock.ISO(x.At), "five_hour", orNone(x.FiveHour), "seven_day", orNone(x.SevenDay),
		"five_resets", x.FiveResets, "status", x.Status)
}

func orNone(f *float64) any {
	if f == nil {
		return nil
	}
	return *f
}

// Compare is `quiet`, `rose`, `inconclusive` (a window turned over between the two) or
// `refused`.
func Compare(a *Sample, b *Sample) string {
	if b.Status == "rejected" {
		return "refused"
	}
	if !pyjson.Equal(a.FiveResets, b.FiveResets) || a.FiveHour == nil || b.FiveHour == nil {
		return "inconclusive"
	}
	if *b.FiveHour > *a.FiveHour+Epsilon {
		return "rose"
	}
	if a.SevenDay != nil && b.SevenDay != nil && pyjson.Equal(a.SevenResets, b.SevenResets) &&
		*b.SevenDay > *a.SevenDay+Epsilon {
		return "rose"
	}
	return "quiet"
}

// isoText is what parse_iso reads of a value: nothing for a falsy one, else its str().
func isoText(v any) string {
	if !pyjson.Truthy(v) {
		return ""
	}
	return py.Str(v)
}

// overlaps is _overlaps: whether a step ran at some point in [t1, t2].
func overlaps(step *pyjson.Object, t1, t2 time.Time) bool {
	if pyjson.Equal(step.Get("conclusion"), "skipped") {
		return false // GitHub skipped it, so it spent nothing
	}
	started := clock.ParseISO(isoText(step.Get("started_at")))
	if started.IsZero() || started.After(t2) {
		return false
	}
	ended := clock.ParseISO(isoText(step.Get("completed_at")))
	return ended.IsZero() || !ended.Before(t1)
}

// RunReader is what partner_spending reads a partner's Actions runs with: a PartnerReader, or a
// test's fake.
type RunReader interface {
	ListRuns(workflow string, limit int) []*pyjson.Object
	ListJobs(runID any) []*pyjson.Object
}

// PartnerSpending is which partner step spent the subscription at some point in [t1, t2], or ""
// (None).
//
// Reads each partner's recent runs and their jobs' steps. An error reading a partner counts as
// not spending, so a rise is never excused on a guess.
func PartnerSpending(partners []*config.Partner, clientFor func(string) RunReader, t1 time.Time, t2 time.Time) string {
	for _, partner := range partners {
		found := ""
		herr.Catch[*herr.GitHubError](func() {
			client := clientFor(partner.Repo)
			for _, workflow := range partner.Workflows.Keys() {
				prefixes := prefixesOf(partner.Workflows.Get(workflow))
				for _, run := range client.ListRuns(workflow, 10) {
					updated := clock.ParseISO(isoText(run.Get("updated_at")))
					if pyjson.Equal(run.Get("status"), "completed") && (updated.IsZero() || updated.Before(t1)) {
						continue
					}
					for _, job := range client.ListJobs(run.Get("id")) {
						for _, step := range pyjson.Objs(job.Get("steps")) {
							name := py.Str(step.GetOr("name", ""))
							if startsWithAny(name, prefixes) && overlaps(step, t1, t2) {
								found = partner.Repo + " " + workflow + ": " + name
								return
							}
						}
					}
				}
			}
		})
		if found != "" {
			return found
		}
	}
	return ""
}

// prefixesOf is a workflow's spending steps as config holds them.
func prefixesOf(v any) []string {
	if s, ok := v.([]string); ok {
		return s
	}
	return pyjson.Strings(v)
}

func startsWithAny(name string, prefixes []string) bool {
	for _, p := range prefixes {
		if strings.HasPrefix(name, p) {
			return true
		}
	}
	return false
}

// Verdict is quiet.Verdict.
type Verdict struct {
	Quiet     bool
	Reason    string
	Samples   []*pyjson.Object // default: field(default_factory=list)
	ExcusedBy string           // default: None ("")
}

// ToDict is Verdict.to_dict.
func (x *Verdict) ToDict() *pyjson.Object {
	samples := make([]any, len(x.Samples))
	for i, s := range x.Samples {
		samples[i] = s
	}
	var excused any
	if x.ExcusedBy != "" {
		excused = x.ExcusedBy
	}
	return pyjson.New("quiet", x.Quiet, "reason", x.Reason, "samples", samples, "excused_by", excused)
}

// WaitForQuiet samples, waits an interval, samples again, until the subscription is quiet or
// the wait is up.
func WaitForQuiet(settings *config.Quiet, ping func() *pyjson.Object, partner func(time.Time, time.Time) string, now func() time.Time, sleep func(float64)) *Verdict {
	interval := time.Duration(settings.IntervalMinutes) * time.Minute
	deadline := now().Add(time.Duration(settings.MaxWaitMinutes) * time.Minute)
	usage := ping()
	first := SampleOf(usage, now())
	if first == nil {
		// No reading at all: nothing may depend on the usage signal being there, so the run goes
		// ahead (bright-bots-harness does the same) rather than waiting every night for ever.
		return &Verdict{Quiet: true, Reason: "the CLI gave no usage reading, so quiet cannot be told; going ahead"}
	}
	samples := []*pyjson.Object{first.ToDict()}
	previous := first
	for !now().Add(interval).After(deadline.Add(30 * time.Second)) {
		sleep(interval.Seconds())
		usage := ping()
		current := SampleOf(usage, now())
		if current == nil {
			continue // this pair cannot be compared; the next reading is compared with `previous`
		}
		samples = append(samples, current.ToDict())
		switch Compare(previous, current) {
		case "quiet":
			return &Verdict{Quiet: true, Reason: "usage held for " + py.Str(settings.IntervalMinutes) + " minutes",
				Samples: samples}
		case "refused":
			return &Verdict{Quiet: false, Reason: "the subscription refused the call (its limit is reached)",
				Samples: samples}
		case "rose":
			if spender := partner(previous.At, current.At); spender != "" {
				return &Verdict{Quiet: true, Reason: "usage rose, but only while the partner bot was spending",
					Samples: samples, ExcusedBy: spender}
			}
		}
		previous = current
	}
	return &Verdict{Quiet: false, Reason: "someone else kept using the subscription for " +
		py.Str(settings.MaxWaitMinutes) + " minutes", Samples: samples}
}

// ActionsReader is the part of a GitHub client a PartnerReader calls (*gh.GitHub has it).
type ActionsReader interface {
	ListRuns(workflow string, status string, limit int, cached bool) []*pyjson.Object
	ListJobs(runID any, cached bool) []*pyjson.Object
}

// PartnerReader reads a partner's public Actions runs: with the job's token first, then without
// one, since a job's own token may not reach another repository.
type PartnerReader struct {
	Clients []ActionsReader // [make(repo, token)] if token else [], then make(repo, "")
}

// NewPartnerReader is PartnerReader(repo, token, factory); a nil factory is gh.GitHub.
func NewPartnerReader(repo string, token string, factory func(repo, token string) ActionsReader /* = None */) *PartnerReader {
	build := factory
	if build == nil {
		build = func(repo, token string) ActionsReader {
			return gh.NewGitHub(repo, token, false, nil, func(s float64) {
				time.Sleep(time.Duration(s * float64(time.Second)))
			}, "")
		}
	}
	x := &PartnerReader{}
	if token != "" {
		x.Clients = append(x.Clients, build(repo, token))
	}
	x.Clients = append(x.Clients, build(repo, ""))
	return x
}

// call is _call: `name` asked of each client in turn, on to the next when GitHub refuses the
// first (401, 403 or 404); any other error is raised, and the last refusal when all refuse.
func (x *PartnerReader) call(name string, fn func(ActionsReader) []*pyjson.Object) []*pyjson.Object {
	var last *herr.GitHubError
	for _, client := range x.Clients {
		var out []*pyjson.Object
		exc, raised := herr.Catch[*herr.GitHubError](func() { out = fn(client) })
		if !raised {
			return out
		}
		if exc.Status != 401 && exc.Status != 403 && exc.Status != 404 {
			herr.Raise(exc)
		}
		last = exc
	}
	if last != nil {
		herr.Raise(last)
	}
	herr.Raise(&herr.GitHubError{Msg: name + ": no client", Status: 0})
	return nil
}

// ListRuns is PartnerReader.list_runs.
func (x *PartnerReader) ListRuns(workflow string, limit int /* = 10 */) []*pyjson.Object {
	return x.call("list_runs", func(c ActionsReader) []*pyjson.Object { return c.ListRuns(workflow, "", limit, false) })
}

// ListJobs is PartnerReader.list_jobs.
func (x *PartnerReader) ListJobs(runID any) []*pyjson.Object {
	return x.call("list_jobs", func(c ActionsReader) []*pyjson.Object { return c.ListJobs(runID, false) })
}
