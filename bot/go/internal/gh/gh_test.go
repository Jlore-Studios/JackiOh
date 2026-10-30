package gh

import (
	"encoding/base64"
	"errors"
	"net"
	"net/http"
	"net/http/httptest"
	"os"
	"path/filepath"
	"reflect"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds the Python's answers for the same inputs (urllib.parse, Python
// 3.13).
func python(t *testing.T, key string) []any {
	t.Helper()
	data, err := os.ReadFile(filepath.Join("testdata", "python.json"))
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	cases := pyjson.Obj(v).List(key)
	if len(cases) == 0 {
		t.Fatalf("no %s cases", key)
	}
	return cases
}

func TestQuoteAsUrllib(t *testing.T) {
	for _, c := range python(t, "quote") {
		row := pyjson.List(c)
		if got := quote(row[0].(string), row[1].(string)); got != row[2] {
			t.Errorf("quote(%q, %q) = %q; Python %q", row[0], row[1], got, row[2])
		}
	}
	for _, c := range python(t, "quote_plus") {
		row := pyjson.List(c)
		if got := quotePlus(row[0].(string), ""); got != row[1] {
			t.Errorf("quotePlus(%q) = %q; Python %q", row[0], got, row[1])
		}
	}
	query := pyjson.New("state", "open", "labels", "bot:build,human", "since", "2026-10-30T01:02:03Z",
		"head", "o:bot/issue 5", "per_page", 100, "page", 2)
	if got, want := urlencode(query), python(t, "urlencode")[0]; got != want {
		t.Errorf("urlencode = %q; Python %q", got, want)
	}
}

// opener is a fake opener: answer gives each request's answer, and every request is kept.
type opener struct {
	asked  []*Request
	answer func(req *Request) (*Response, error)
}

func (o *opener) open(req *Request, timeout time.Duration) (*Response, error) {
	o.asked = append(o.asked, req)
	return o.answer(req)
}

func jsonAnswer(v any) *Response {
	return &Response{Header: http.Header{}, Body: []byte(pyjson.Dumps(v))}
}

func TestHeadersAndBody(t *testing.T) {
	o := &opener{answer: func(*Request) (*Response, error) { return jsonAnswer(pyjson.New("ok", true)), nil }}
	client := NewGitHub("o/r", "tok", false, o.open, nil, "")
	client.AddLabels(5, []string{"", "patch"})
	req := o.asked[0]
	if req.Method != "POST" || req.URL != "https://api.github.com/repos/o/r/issues/5/labels" ||
		string(req.Body) != `{"labels": ["patch"]}` {
		t.Fatalf("%s %s", req.Method, req.URL)
	}
	for k, v := range map[string]string{
		"User-Agent": "jackioh-night-bot/1.0.0", "Accept": "application/vnd.github+json",
		"X-Github-Api-Version": "2022-11-28", "Authorization": "Bearer tok", "Content-Type": "application/json",
	} {
		if got := req.Header.Get(k); got != v {
			t.Errorf("%s: %q; want %q", k, got, v)
		}
	}
	client.RepoInfo()
	if get := o.asked[1]; get.Body != nil || get.Header.Get("Content-Type") != "" {
		t.Errorf("a GET sends no body: %q", get.Body)
	}
	if NewGitHub("o/r", "", false, o.open, nil, "").headers("x").Get("Authorization") != "" {
		t.Error("no token, no Authorization")
	}
}

func TestDryRunRecordsWritesAndSendsNothing(t *testing.T) {
	o := &opener{answer: func(*Request) (*Response, error) { t.Fatal("sent"); return nil, nil }}
	client := NewGitHub("o/r", "tok", true, o.open, nil, "")
	client.Lock(5)
	if got := client.Graphql("query", pyjson.New("id", "x")); got.Len() != 0 {
		t.Fatal("a dry-run GraphQL answers {}")
	}
	if len(client.Writes) != 2 || client.Writes[0].Str("method") != "PUT" ||
		client.Writes[0].Str("path") != "/repos/o/r/issues/5/lock" || pyjson.Dumps(client.Writes[0].Get("body")) != "{}" {
		t.Fatalf("writes = %v", pyjson.Dumps(client.Writes))
	}
}

// test_hardening.py's test_a_refused_bot_token_falls_back_to_the_actions_token.
func TestARefusedBotTokenFallsBackToTheActionsToken(t *testing.T) {
	var seen []string
	o := &opener{answer: func(req *Request) (*Response, error) {
		token := req.Header.Get("Authorization")
		seen = append(seen, token)
		if token == "Bearer bot-token" {
			return nil, &HTTPError{URL: req.URL, Code: 401, Msg: "Bad credentials", Body: []byte("{}")}
		}
		return jsonAnswer(pyjson.New("ok", true)), nil
	}}
	client := NewGitHub("o/r", "bot-token", false, o.open, nil, "actions-token")
	if got := client.Request("GET", "/x", nil, "", false, false); pyjson.Dumps(got) != `{"ok": true}` {
		t.Fatalf("answer %v", got)
	}
	client.Request("GET", "/y", nil, "", false, false)
	if !reflect.DeepEqual(seen, []string{"Bearer bot-token", "Bearer actions-token", "Bearer actions-token"}) {
		t.Fatalf("tokens %q", seen)
	}
}

// test_hardening.py's test_a_cached_get_sends_its_etag_and_keeps_the_answer_github_says_is_unchanged.
func TestACachedGetSendsItsETag(t *testing.T) {
	Cached.Clear()
	t.Cleanup(Cached.Clear)
	var sent []string
	o := &opener{answer: func(req *Request) (*Response, error) {
		sent = append(sent, req.Header.Get("If-None-Match"))
		if sent[len(sent)-1] == `"v1"` {
			return nil, &HTTPError{URL: req.URL, Code: 304, Msg: "Not Modified"}
		}
		resp := jsonAnswer(pyjson.New("jobs", []any{pyjson.New("id", 1)}))
		resp.Header.Set("ETag", `"v1"`)
		return resp, nil
	}}
	client := NewGitHub("o/r", "token-for-etags", false, o.open, nil, "")
	for i := 0; i < 2; i++ {
		if got := client.ListJobs(5, true); len(got) != 1 || got[0].Int("id") != 1 {
			t.Fatalf("jobs %v", got)
		}
	}
	client.ListJobs(5, false)
	if !reflect.DeepEqual(sent, []string{"", `"v1"`, ""}) {
		t.Fatalf("If-None-Match %q", sent)
	}
	if !strings.HasSuffix(o.asked[0].URL, "/repos/o/r/actions/runs/5/jobs?per_page=100&page=1") {
		t.Fatalf("url %s", o.asked[0].URL)
	}
}

func TestTheCacheKeepsTheNewest(t *testing.T) {
	Cached.Clear()
	t.Cleanup(Cached.Clear)
	keep := CachedKeep
	CachedKeep = 2
	t.Cleanup(func() { CachedKeep = keep })
	for _, k := range []string{"a", "b", "a", "c"} {
		Cached.put([2]string{"t", k}, [2]string{"e", k})
	}
	_, hasA := Cached.get([2]string{"t", "a"})
	_, hasB := Cached.get([2]string{"t", "b"})
	if Cached.Len() != 2 || !hasA || hasB {
		t.Fatalf("kept %v", Cached.keys)
	}
}

func TestRetriesAndRetryAfter(t *testing.T) {
	var slept []float64
	n := 0
	o := &opener{answer: func(req *Request) (*Response, error) {
		n++
		switch n {
		case 1:
			return nil, &HTTPError{Code: 502, Msg: "Bad Gateway"}
		case 2:
			h := http.Header{}
			h.Set("Retry-After", "7")
			return nil, &HTTPError{Code: 429, Msg: "Too Many Requests", Header: h}
		}
		return jsonAnswer([]any{}), nil
	}}
	client := NewGitHub("o/r", "t", false, o.open, func(s float64) { slept = append(slept, s) }, "")
	client.Request("GET", "/x", nil, "", false, false)
	if !reflect.DeepEqual(slept, []float64{2, 7}) {
		t.Fatalf("slept %v", slept)
	}
	slept, n = nil, 1
	o.answer = func(req *Request) (*Response, error) {
		n++
		if n < 4 {
			return nil, &URLError{Reason: "timed out"}
		}
		return jsonAnswer([]any{}), nil
	}
	client.Request("GET", "/x", nil, "", false, false)
	if !reflect.DeepEqual(slept, []float64{2, 4}) {
		t.Fatalf("slept %v", slept)
	}
}

func TestPaginate(t *testing.T) {
	o := &opener{answer: func(req *Request) (*Response, error) {
		if strings.Contains(req.URL, "page=1") {
			return jsonAnswer(pyjson.New("workflow_runs", []any{1, 2})), nil
		}
		return jsonAnswer(pyjson.New("workflow_runs", []any{3})), nil
	}}
	client := NewGitHub("o/r", "t", false, o.open, nil, "")
	got := client.Paginate("/repos/o/r/actions/runs", pyjson.New("status", "in progress", "per_page", 2), "workflow_runs", 10, false)
	if !reflect.DeepEqual(got, []any{1, 2, 3}) {
		t.Fatalf("items %v", got)
	}
	want := []string{
		"https://api.github.com/repos/o/r/actions/runs?status=in+progress&per_page=2&page=1",
		"https://api.github.com/repos/o/r/actions/runs?status=in+progress&per_page=2&page=2",
	}
	if len(o.asked) != 2 || o.asked[0].URL != want[0] || o.asked[1].URL != want[1] {
		t.Fatalf("asked %v %v", o.asked[0].URL, o.asked[len(o.asked)-1].URL)
	}
	if got := client.Paginate("/x", nil, "workflow_runs", 1, false); !reflect.DeepEqual(got, []any{1}) {
		t.Fatalf("limit: %v", got)
	}
}

// test_basics.py's test_a_file_over_1_mb_is_read_from_its_blob.
func TestAFileOver1MBIsReadFromItsBlob(t *testing.T) {
	big := strings.Repeat("é", 600_000)
	o := &opener{answer: func(req *Request) (*Response, error) {
		if strings.Contains(req.URL, "/contents/") {
			return jsonAnswer(pyjson.New("type", "file", "encoding", "none", "content", "", "size", len(big), "sha", "abc123")), nil
		}
		return jsonAnswer(pyjson.New("sha", "abc123", "content", base64.StdEncoding.EncodeToString([]byte(big)))), nil
	}}
	text, sha := NewGitHub("o/r", "token", false, o.open, nil, "").GetFile("state.json", "bot-state")
	if text != big || sha != "abc123" {
		t.Fatalf("got %d chars, sha %q", len(text), sha)
	}
	if !strings.HasSuffix(o.asked[len(o.asked)-1].URL, "/repos/o/r/git/blobs/abc123") {
		t.Fatal(o.asked[len(o.asked)-1].URL)
	}
	if !strings.HasSuffix(o.asked[0].URL, "/repos/o/r/contents/state.json?ref=bot-state") {
		t.Fatal(o.asked[0].URL)
	}
}

// test_controller.py's test_an_artifact_is_fetched_from_its_redirect_without_the_token.
func TestAnArtifactIsFetchedFromItsRedirectWithoutTheToken(t *testing.T) {
	var asked []*Request
	build, open := BuildOpener, Urlopen
	t.Cleanup(func() { BuildOpener, Urlopen = build, open })
	BuildOpener = func() Opener {
		return func(req *Request, timeout time.Duration) (*Response, error) {
			asked = append(asked, req)
			h := http.Header{}
			h.Set("Location", "https://blob.example/work.zip")
			return nil, &HTTPError{URL: req.URL, Code: 302, Msg: "Found", Header: h}
		}
	}
	Urlopen = func(req *Request, timeout time.Duration) (*Response, error) {
		asked = append(asked, req)
		return &Response{Body: []byte("zip bytes")}, nil
	}
	client := NewGitHub("jgoetzmann/JackiOh", "secret-token", false, nil, nil, "")
	if got := client.DownloadArtifact(7); string(got) != "zip bytes" {
		t.Fatalf("got %q", got)
	}
	if !strings.HasSuffix(asked[0].URL, "/repos/jgoetzmann/JackiOh/actions/artifacts/7/zip") ||
		asked[0].Header.Get("Authorization") != "Bearer secret-token" {
		t.Fatal("the API call carries the token")
	}
	if asked[1].URL != "https://blob.example/work.zip" || len(asked[1].Header) != 0 {
		t.Fatal("the storage URL goes without a header")
	}
}

func TestRequiredChecksAndQueueRemoval(t *testing.T) {
	answer := pyjson.New("protected", true, "protection", pyjson.New("required_status_checks",
		pyjson.New("contexts", []any{"rust"}, "checks", []any{pyjson.New("context", "web"), pyjson.New("app_id", 1)})))
	o := &opener{answer: func(req *Request) (*Response, error) { return jsonAnswer(answer), nil }}
	client := NewGitHub("o/r", "t", false, o.open, nil, "")
	if got := client.RequiredChecks("ma in"); !reflect.DeepEqual(got, map[string]bool{"rust": true, "web": true, "None": true}) {
		t.Fatalf("checks %v", got)
	}
	if !strings.HasSuffix(o.asked[0].URL, "/branches/ma%20in") {
		t.Fatal(o.asked[0].URL)
	}
	answer = pyjson.New("protected", true)
	if got := client.RequiredChecks("main"); got == nil || len(got) != 0 {
		t.Fatalf("a protected branch with none: %v", got)
	}
	answer = pyjson.New("data", pyjson.New("node", pyjson.New("isInMergeQueue", false, "timelineItems",
		pyjson.New("nodes", []any{pyjson.New("__typename", "RemovedFromMergeQueueEvent", "reason", "CI failed",
			"actor", pyjson.New("__typename", "Bot", "login", "github-merge-queue")),
		}))))
	if got := client.QueueRemoval("N"); !reflect.DeepEqual(got, map[string]string{"actor": "github-merge-queue", "type": "Bot", "reason": "CI failed"}) {
		t.Fatalf("removal %v", got)
	}
	answer = pyjson.New("data", pyjson.New("node", nil))
	if got := client.QueueRemoval("N"); got != nil {
		t.Fatalf("removal %v", got)
	}
}

// The real opener, against local servers: urllib's redirect rules and its exceptions.
func TestTheOpenerFollowsRedirectsAsUrllib(t *testing.T) {
	var auth string
	target := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		auth = r.Header.Get("Authorization")
		w.Write([]byte(`{"method": "` + r.Method + `"}`))
	}))
	defer target.Close()
	front := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		if r.Header.Get("Accept-Encoding") != "identity" || r.Header.Get("Connection") != "close" && !r.Close {
			t.Errorf("headers %v", r.Header)
		}
		http.Redirect(w, r, strings.Replace(target.URL, "127.0.0.1", "localhost", 1)+"/x", http.StatusFound)
	}))
	defer front.Close()
	header := http.Header{}
	header.Set("Authorization", "Bearer tok")
	resp, err := Urlopen(&Request{Method: "GET", URL: front.URL, Header: header}, 5*time.Second)
	if err != nil || string(resp.Body) != `{"method": "GET"}` || auth != "Bearer tok" {
		t.Fatalf("GET: %v %v %q", resp, err, auth)
	}
	_, err = Urlopen(&Request{Method: "PATCH", URL: front.URL, Header: header, Body: []byte("{}")}, 5*time.Second)
	var he *HTTPError
	if !errors.As(err, &he) || he.Code != 302 || he.Msg != "Found" {
		t.Fatalf("a PATCH's redirect is an HTTPError: %v", err)
	}
	_, err = BuildOpener()(&Request{Method: "GET", URL: front.URL, Header: header}, 5*time.Second)
	if !errors.As(err, &he) || he.Code != 302 || !strings.HasSuffix(he.Header.Get("Location"), "/x") {
		t.Fatalf("no redirect followed: %v", err)
	}
}

func TestTheOpenersExceptions(t *testing.T) {
	slow := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		time.Sleep(300 * time.Millisecond)
	}))
	defer slow.Close()
	_, err := Urlopen(&Request{Method: "GET", URL: slow.URL}, 50*time.Millisecond)
	var te *TimeoutError
	if !errors.As(err, &te) || err.Error() != "timed out" {
		t.Fatalf("a slow answer: %v", err)
	}
	listener, _ := net.Listen("tcp", "127.0.0.1:0")
	addr := listener.Addr().String()
	listener.Close()
	_, err = Urlopen(&Request{Method: "GET", URL: "http://" + addr}, time.Second)
	if err == nil || err.Error() != "<urlopen error [Errno 111] Connection refused>" {
		t.Fatalf("a refused connection: %v", err)
	}
	notFound := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(404)
		w.Write([]byte(`{"message": "Not Found"}`))
	}))
	defer notFound.Close()
	_, err = Urlopen(&Request{Method: "GET", URL: notFound.URL}, time.Second)
	var he *HTTPError
	if !errors.As(err, &he) || err.Error() != "HTTP Error 404: Not Found" || string(he.Body) != `{"message": "Not Found"}` {
		t.Fatalf("a 404: %v", err)
	}
}
