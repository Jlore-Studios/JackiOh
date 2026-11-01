package runner

import (
	"os"
	"path/filepath"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds what the Python harness answered for each input below
// (bot/harness/runner.py, Python 3.13): the parsers must answer the same, byte for byte.
func python(t *testing.T, key string) [][]any {
	t.Helper()
	raw, err := os.ReadFile(filepath.Join("testdata", "python.json"))
	if err != nil {
		t.Fatal(err)
	}
	all, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	var cases [][]any
	for _, c := range pyjson.List(all.(*pyjson.Object).Get(key)) {
		cases = append(cases, pyjson.List(c))
	}
	if len(cases) == 0 {
		t.Fatalf("no cases for %s", key)
	}
	return cases
}

// same is whether got is the JSON value Python gave, as json.dumps writes both.
func same(t *testing.T, what string, got, want any) {
	t.Helper()
	if g, w := pyjson.Dumps(got), pyjson.Dumps(want); g != w {
		t.Errorf("%s:\n got %s\nwant %s", what, g, w)
	}
}

// orNil is a reading as Python returned it: None for a nil object.
func orNil(o *pyjson.Object) any {
	if o == nil {
		return nil
	}
	return o
}

func TestStderrError(t *testing.T) {
	for _, c := range python(t, "stderr") {
		same(t, py.Repr(c[0]), StderrError(c[0].(string), c[1].(string)), c[2])
	}
}

func TestParkFor(t *testing.T) {
	for _, c := range python(t, "park") {
		same(t, py.Repr(c[0]), ParkFor(c[0].(string)), c[1])
	}
}

func TestMuseReset(t *testing.T) {
	central := time.FixedZone("", -5*3600)
	for key, now := range map[string]time.Time{
		"muse_reset":     time.Date(2026, 10, 4, 10, 30, 0, 0, central),
		"muse_reset_utc": time.Date(2026, 10, 4, 15, 30, 0, 0, time.UTC),
	} {
		for _, c := range python(t, key) {
			var got any
			if s := MuseReset(c[0].(string), now); s != "" {
				got = s
			}
			same(t, key+" "+py.Repr(c[0]), got, c[1])
		}
	}
}

func TestMuseResetReadsTheOffsetOfTheMoment(t *testing.T) {
	// Python reads datetime.now().astimezone(): today's offset, kept for a reset after the clocks
	// change (the US went back an hour on 2026-11-01).
	chicago, err := time.LoadLocation("America/Chicago")
	if err != nil {
		t.Skip("no zone database")
	}
	now := time.Date(2026, 10, 31, 12, 0, 0, 0, chicago)
	if got := MuseReset("Nov 2 at 12:00 PM", now); got != "2026-11-02T17:00:00Z" {
		t.Errorf("got %s", got)
	}
}

func TestParseMuseUsage(t *testing.T) {
	now := time.Date(2026, 10, 4, 10, 30, 0, 0, time.FixedZone("", -5*3600))
	for _, c := range python(t, "muse_usage") {
		same(t, py.Repr(c[0]), orNil(ParseMuseUsage(c[0].(string), now)), c[1])
	}
}

func TestParseAgyUsage(t *testing.T) {
	for _, c := range python(t, "agy") {
		same(t, py.Repr(c[0])+" "+c[1].(string), orNil(ParseAgyUsage(c[0].(string), c[1].(string))), c[2])
	}
}

func TestUsageFromEvent(t *testing.T) {
	for _, c := range python(t, "usage_event") {
		same(t, pyjson.Dumps(c[0]), orNil(UsageFromEvent(c[0].(*pyjson.Object))), c[1])
	}
}

func TestParseStream(t *testing.T) {
	for _, c := range python(t, "stream") {
		lines := pyjson.Strings(c[0])
		result, usage := ParseStream(lines)
		same(t, pyjson.Dumps(c[0]), []any{orNil(result), orNil(usage)}, c[1])
	}
}

func TestTokensAndCost(t *testing.T) {
	for _, c := range python(t, "tokens") {
		same(t, pyjson.Dumps(c[0]), orNil(TokensFromBlock(c[0])), c[1])
	}
	for _, c := range python(t, "cost") {
		var got any
		if f := CostFromResult(pyjson.Obj(c[0])); f != nil {
			got = *f
		}
		same(t, pyjson.Dumps(c[0]), got, c[1])
	}
}

func TestClip(t *testing.T) {
	for _, c := range python(t, "clip") {
		n, _ := pyjson.AsInt(c[1])
		same(t, pyjson.Dumps(c[0]), Clip(c[0], n), c[2])
	}
}

func TestWords(t *testing.T) {
	for _, c := range python(t, "rate_words") {
		same(t, "rate "+c[0].(string), RateLimitWords.MatchString(c[0].(string)), c[1])
	}
	for _, c := range python(t, "auth_words") {
		same(t, "auth "+c[0].(string), AuthWords.MatchString(c[0].(string)), c[1])
	}
}

func TestRunResultInfraAndRateLimited(t *testing.T) {
	auth := NewRunResult(false, "")
	auth.ExitCode, auth.Error = 1, "Invalid API key · Please run /login"
	if !auth.Infra() || auth.RateLimited() {
		t.Error("an auth failure is infrastructure, not a rate limit")
	}
	turnsOut := NewRunResult(false, "")
	turnsOut.ExitCode, turnsOut.Error = 1, "max turns reached"
	if turnsOut.Infra() {
		t.Error("max turns is no infrastructure")
	}
	missing := NewRunResult(false, "")
	missing.ExitCode = ExitNotFound
	if !missing.Infra() {
		t.Error("a missing CLI is infrastructure")
	}
	refused := NewRunResult(false, "")
	refused.Usage = pyjson.New("status", "rejected")
	if !refused.RateLimited() {
		t.Error("a rejected reading is a rate limit")
	}
	refused.OK = true
	if refused.RateLimited() || refused.Infra() {
		t.Error("a call that worked is neither")
	}
	parked := NewRunResult(true, "")
	parked.ResetAt = "+PT60M"
	if !parked.RateLimited() {
		t.Error("a reset is a rate limit")
	}
}

func TestCodexUsage(t *testing.T) {
	home := t.TempDir()
	day := filepath.Join(home, "sessions", "2026", "10", "02")
	if err := os.MkdirAll(day, 0o777); err != nil {
		t.Fatal(err)
	}
	line := `{"type": "event_msg", "payload": {"type": "token_count", "rate_limits": {"primary": ` +
		`{"used_percent": 42.0, "window_minutes": 300, "resets_at": 1790710800}, "secondary": ` +
		`{"used_percent": 61, "window_minutes": 10080, "resets_at": 1791190800}}}}` + "\n"
	if err := os.WriteFile(filepath.Join(day, "rollout-1.jsonl"), []byte("noise\n"+line), 0o666); err != nil {
		t.Fatal(err)
	}
	got := CodexUsage(home, float64(time.Now().Unix()-60))
	same(t, "codex", got, pyjson.New("status", "allowed",
		"five_hour", pyjson.New("utilization", 0.42, "resets_at", "2026-09-29T19:40:00Z"),
		"seven_day", pyjson.New("utilization", 0.61, "resets_at", "2026-10-05T09:00:00Z")))
	if CodexUsage(home, float64(time.Now().Unix()+60)) != nil {
		t.Error("a session log older than the call is not read")
	}
	if CodexUsage(filepath.Join(home, "none"), 0) != nil {
		t.Error("no sessions, no reading")
	}
}

func TestWithSuffix(t *testing.T) {
	for in, want := range map[string]string{
		"t/01-build.jsonl":    "t/01-build.raw",
		"/w/.claude-b.jsonl":  "/w/.claude-b.raw",
		".bashrc":             ".bashrc.raw",
		"a.b/c":               "a.b/c.raw",
		"x/archive.tar.gz":    "x/archive.tar.raw",
		"trailing.":           "trailing..raw",
		"/abs/01-review.json": "/abs/01-review.raw",
	} {
		if got := withSuffix(in, ".raw"); got != want {
			t.Errorf("%s: got %s, want %s", in, got, want)
		}
	}
}

func TestFakeRunner(t *testing.T) {
	answered := 0
	first := func(*RunRequest) *RunResult { answered++; return NewRunResult(true, "one") }
	second := func(*RunRequest) *RunResult { answered++; return NewRunResult(true, "two") }
	runner := NewFakeRunner(map[string][]Handler{"build": {first, second}})
	texts := []string{}
	for range 3 {
		texts = append(texts, runner.Run(&RunRequest{Role: "build"}).Text)
	}
	if pyjson.Dumps(texts) != `["one", "two", "two"]` || answered != 3 {
		t.Errorf("got %v", texts)
	}
	if got := runner.Run(&RunRequest{Role: "plan"}); !got.OK || got.Text != FakePlan {
		t.Errorf("plan: %+v", got)
	}
	if got := runner.Run(&RunRequest{Role: "review"}); got.OK || got.Error != "fake runner has no handler for review" {
		t.Errorf("review: %+v", got)
	}
	if len(runner.Calls) != 5 || runner.Calls[4].Role != "review" {
		t.Errorf("calls: %d", len(runner.Calls))
	}
}
