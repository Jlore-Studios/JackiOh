package quiet

import (
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

var t0 = time.Date(2026, 9, 30, 3, 0, 0, 0, time.UTC)

func f(v float64) *float64 { return &v }

// The cases of bot/tests/test_quiet.py's CompareTests.
func TestCompare(t *testing.T) {
	s := func(five, seven float64, resets any, status string) *Sample {
		return &Sample{At: t0, FiveHour: f(five), SevenDay: f(seven), FiveResets: resets, SevenResets: "w", Status: status}
	}
	for _, c := range []struct {
		a, b *Sample
		want string
	}{
		{s(0.10, 0.5, "r1", "allowed"), s(0.10, 0.5, "r1", "allowed"), "quiet"},
		{s(0.10, 0.5, "r1", "allowed"), s(0.11, 0.5, "r1", "allowed"), "rose"},
		{s(0.10, 0.50, "r1", "allowed"), s(0.10, 0.51, "r1", "allowed"), "rose"},
		{s(0.40, 0.5, "r1", "allowed"), s(0.02, 0.5, "r2", "allowed"), "inconclusive"},
		{s(0.10, 0.5, "r1", "allowed"), s(0.10, 0.5, "r1", "rejected"), "refused"},
		{s(0.10, 0.5, "r1", "allowed"), s(0.1005, 0.5, "r1", "allowed"), "quiet"},
		// None and "" are not the same reset, as in Python.
		{s(0.10, 0.5, nil, "allowed"), s(0.10, 0.5, "", "allowed"), "inconclusive"},
	} {
		if got := Compare(c.a, c.b); got != c.want {
			t.Errorf("got %s, want %s", got, c.want)
		}
	}
	noFive := &Sample{At: t0, SevenDay: f(0.5), Status: "allowed"}
	if got := Compare(noFive, noFive); got != "inconclusive" {
		t.Errorf("no 5-hour window: %s", got)
	}
}

func TestSampleOf(t *testing.T) {
	usage := pyjson.New(
		"five_hour", pyjson.New("utilization", 1, "resets_at", "2026-09-30T05:00:00Z"),
		"seven_day", pyjson.New("utilization", "high", "resets_at", nil),
		"status", nil)
	s := SampleOf(usage, t0)
	if s == nil || *s.FiveHour != 1.0 || s.SevenDay != nil || s.FiveResets != "2026-09-30T05:00:00Z" ||
		s.SevenResets != nil || s.Status != "None" {
		t.Errorf("got %+v", s)
	}
	if SampleOf(nil, t0) != nil || SampleOf(pyjson.New("status", "allowed"), t0) != nil {
		t.Error("no window, no sample")
	}
	if got := SampleOf(pyjson.New("seven_day", pyjson.New("utilization", 0.5)), t0); got == nil || got.Status != "" {
		t.Errorf("one window is a sample: %+v", got)
	}
}

func TestNoReadingGoesAhead(t *testing.T) {
	slept := 0
	v := WaitForQuiet(&config.Quiet{Enabled: true, IntervalMinutes: 10, MaxWaitMinutes: 40},
		func() *pyjson.Object { return nil }, func(time.Time, time.Time) string { return "" },
		func() time.Time { return t0 }, func(float64) { slept++ })
	if !v.Quiet || v.Reason != "the CLI gave no usage reading, so quiet cannot be told; going ahead" || slept != 0 {
		t.Errorf("got %+v", v)
	}
	if got := pyjson.Dumps(v.ToDict()); got != `{"quiet": true, "reason": "the CLI gave no usage reading, so quiet cannot be told; going ahead", "samples": [], "excused_by": null}` {
		t.Errorf("to_dict %s", got)
	}
}

type client struct {
	token string
	fail  int
	made  *[]string
}

func (c *client) ListRuns(workflow string, status string, limit int, cached bool) []*pyjson.Object {
	if c.token != "" {
		herr.Raise(&herr.GitHubError{Msg: "Resource not accessible by integration", Status: c.fail})
	}
	return []*pyjson.Object{pyjson.New("id", 7, "limit", limit)}
}

func (c *client) ListJobs(runID any, cached bool) []*pyjson.Object {
	herr.Raise(&herr.GitHubError{Msg: "Not Found", Status: 404})
	return nil
}

// test_quiet.py's test_the_reader_falls_back_to_no_token, and what it does not fall back on.
func TestPartnerReader(t *testing.T) {
	var made []string
	factory := func(fail int) func(repo, token string) ActionsReader {
		return func(repo, token string) ActionsReader {
			made = append(made, token)
			return &client{token: token, fail: fail, made: &made}
		}
	}
	reader := NewPartnerReader("x/y", "actions-token", factory(403))
	if got := pyjson.Dumps(reader.ListRuns("implement.yml", 10)); got != `[{"id": 7, "limit": 10}]` {
		t.Errorf("runs %s", got)
	}
	if pyjson.Dumps(made) != `["actions-token", ""]` {
		t.Errorf("made %v", made)
	}
	exc, raised := herr.Catch[*herr.GitHubError](func() { reader.ListJobs(7) })
	if !raised || exc.Status != 404 {
		t.Errorf("the last refusal is raised: %v", exc)
	}
	broken := NewPartnerReader("x/y", "actions-token", factory(500))
	exc, raised = herr.Catch[*herr.GitHubError](func() { broken.ListRuns("implement.yml", 10) })
	if !raised || exc.Status != 500 {
		t.Errorf("another error is raised at once: %v", exc)
	}
	made = nil
	NewPartnerReader("x/y", "", factory(403))
	if pyjson.Dumps(made) != `[""]` {
		t.Errorf("without a token, one client: %v", made)
	}
}
