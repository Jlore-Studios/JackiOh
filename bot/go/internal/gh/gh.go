// Package gh is bot/harness/gh.py.
//
// The GitHub client. The only module that sends a token or writes to GitHub.
//
// Every body the harness posts is redacted and carries `config.MARKER`. In dry-run mode every
// write is recorded in `writes` and answered with a stand-in, and nothing is sent.
package gh

import (
	"errors"
	"fmt"
	"net/http"
	"strconv"
	"strings"
	"sync"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/config"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/redact"
)

// version is the harness's `__version__` (bot/harness/__init__.py).
const version = "1.0.0"

// API is the REST root.
var API = "https://api.github.com"

// UserAgent is the User-Agent of every request.
var UserAgent = "jackioh-night-bot/" + version

// Retries is how many times a request is sent before it fails.
var Retries = 3

// BackoffSeconds is the wait after the first failed attempt; the n-th waits n times it.
var BackoffSeconds = 2.0

// Cached holds the answers to conditional GETs (`request`'s `cached`), by token and URL: GitHub
// answers one whose ETag still matches with 304 Not Modified, which costs no rate limit, so the
// status loop's read of every running job every two minutes costs a call only for jobs that
// changed. Kept for the process, since the loop builds a client each tick; the newest
// `CachedKeep`. (Python's `_CACHED`.)
var Cached = &ETagCache{vals: map[[2]string][2]string{}}

// CachedKeep is how many answers Cached keeps.
var CachedKeep = 200

// ETagCache is `_CACHED`: a dict from (token, URL) to (ETag, text) whose oldest entry goes first.
type ETagCache struct {
	mu   sync.Mutex
	keys [][2]string
	vals map[[2]string][2]string
}

// Len is len(_CACHED).
func (c *ETagCache) Len() int {
	c.mu.Lock()
	defer c.mu.Unlock()
	return len(c.keys)
}

// Clear is _CACHED.clear().
func (c *ETagCache) Clear() {
	c.mu.Lock()
	defer c.mu.Unlock()
	c.keys, c.vals = nil, map[[2]string][2]string{}
}

func (c *ETagCache) get(key [2]string) ([2]string, bool) {
	c.mu.Lock()
	defer c.mu.Unlock()
	v, ok := c.vals[key]
	return v, ok
}

// put is `_CACHED.pop(key, None); _CACHED[key] = value`, then the oldest dropped down to
// CachedKeep.
func (c *ETagCache) put(key, value [2]string) {
	c.mu.Lock()
	defer c.mu.Unlock()
	if _, ok := c.vals[key]; ok {
		for i, k := range c.keys {
			if k == key {
				c.keys = append(c.keys[:i:i], c.keys[i+1:]...)
				break
			}
		}
	}
	c.keys = append(c.keys, key)
	c.vals[key] = value
	for len(c.keys) > CachedKeep {
		delete(c.vals, c.keys[0])
		c.keys = c.keys[1:]
	}
}

// ValueError is Python's ValueError as the client raises it: a 2xx answer that is not JSON
// (json.JSONDecodeError), or a Retry-After that is not a number (float()). Callers that catch
// ValueError catch this.
type ValueError struct{ Msg string }

func (e *ValueError) Error() string { return e.Msg }

// WithMarker is `body`, redacted, with the marker once at the end.
func WithMarker(body string) string {
	text := redact.Redact(body, nil)
	marker := config.Marker
	if strings.Contains(text, marker) {
		return text
	}
	if py.Strip(text) != "" {
		return text + "\n\n" + marker
	}
	return marker
}

func parsed(text string, raw bool) any {
	if raw {
		return text
	}
	if py.Strip(text) == "" {
		return pyjson.New()
	}
	v, err := pyjson.LoadsString(text)
	if err != nil {
		herr.Raise(&ValueError{Msg: err.Error()})
	}
	return v
}

// GitHub is a small REST and GraphQL client for one repository.
type GitHub struct {
	Repo  string
	Token string
	// FallbackToken is used for the rest of the run when `token` is refused (a revoked or
	// expired bot token), so replies still go out, as the Actions bot.
	FallbackToken string
	DryRun        bool
	// Open is the opener every request goes through (Python's `_open`).
	Open Opener
	// Sleep waits between attempts, in seconds (Python's `_sleep`).
	Sleep  func(float64)
	Writes []*pyjson.Object
}

// NewGitHub is GitHub(...). A nil opener is Urlopen, read now, and a nil sleep is time.sleep.
func NewGitHub(repo string, token string, dryRun bool /* = False */, opener Opener /* = None */, sleep func(float64) /* = time.sleep */, fallbackToken string /* = "" */) *GitHub {
	if opener == nil {
		opener = Urlopen
	}
	if sleep == nil {
		sleep = timeSleep
	}
	return &GitHub{
		Repo:          repo,
		Token:         token,
		FallbackToken: fallbackToken,
		DryRun:        dryRun,
		Open:          opener,
		Sleep:         sleep,
		Writes:        []*pyjson.Object{},
	}
}

func timeSleep(seconds float64) { time.Sleep(time.Duration(seconds * float64(time.Second))) }

// ------------------------------------------------------------------ transport

func (x *GitHub) headers(accept string /* = None */) http.Header {
	if accept == "" {
		accept = "application/vnd.github+json"
	}
	headers := http.Header{}
	headers.Set("User-Agent", UserAgent)
	headers.Set("Accept", accept)
	headers.Set("X-GitHub-Api-Version", "2022-11-28")
	if x.Token != "" {
		headers.Set("Authorization", "Bearer "+x.Token)
	}
	return headers
}

// URL is a path's full URL: an https:// URL as it is, anything else under API.
func (x *GitHub) URL(path string) string {
	if strings.HasPrefix(path, "https://") {
		return path
	}
	if strings.HasPrefix(path, "/") {
		return API + path
	}
	return API + "/" + path
}

// Request is one request, retried on 5xx, connection errors and secondary rate limits. A
// `cached` GET is conditional: its last answer is kept, and given again when GitHub says it has
// not changed (`_CACHED`). `body` nil sends none; `accept` "" is the API's JSON.
func (x *GitHub) Request(method string, path string, body any /* = None */, accept string /* = None */, raw bool /* = False */, cached bool /* = False */) any {
	if method != "GET" && x.DryRun {
		x.Writes = append(x.Writes, pyjson.New("method", method, "path", path, "body", body))
		return pyjson.New()
	}
	url := x.URL(path)
	var data []byte
	if body != nil {
		data = []byte(pyjson.Dumps(body))
	}
	var last error
	for attempt := 0; attempt < Retries; attempt++ {
		req := &Request{Method: method, URL: url, Header: x.headers(accept), Body: data}
		if data != nil {
			req.Header.Set("Content-Type", "application/json")
		}
		key := [2]string{x.Token, url}
		if cached {
			if kept, ok := Cached.get(key); ok {
				req.Header.Set("If-None-Match", kept[0])
			}
		}
		resp, err := x.Open(req, 60*time.Second)
		if err == nil {
			text := py.DecodeReplace(resp.Body)
			etag := resp.Header.Get("ETag")
			if cached && etag != "" {
				Cached.put(key, [2]string{etag, text})
			}
			return parsed(text, raw)
		}
		var exc *HTTPError
		if !errors.As(err, &exc) {
			if !isTransportError(err) {
				herr.Raise(err)
			}
			last = err
			if attempt+1 < Retries {
				x.Sleep(BackoffSeconds * float64(attempt+1))
			}
			continue
		}
		if exc.Code == 304 && cached {
			if kept, ok := Cached.get(key); ok {
				return parsed(kept[1], raw)
			}
		}
		detail := ""
		if exc.Body != nil {
			detail = py.DecodeReplace(exc.Body)
		}
		if exc.Code == 401 && x.FallbackToken != "" && x.Token != x.FallbackToken {
			x.Token = x.FallbackToken
			continue
		}
		retryAfter := exc.Header.Get("Retry-After")
		limited := (exc.Code == 403 || exc.Code == 429) &&
			(retryAfter != "" || strings.Contains(strings.ToLower(detail), "secondary rate limit"))
		// A POST that failed with a 5xx may still have happened (a comment posted, a pull
		// request opened), so only a rate-limit refusal is retried for it.
		retryable := limited || (exc.Code >= 500 && method != "POST")
		if retryable && attempt+1 < Retries {
			wait := BackoffSeconds * float64(attempt+1)
			if retryAfter != "" {
				f, err := py.Float(retryAfter)
				if err != nil {
					herr.Raise(&ValueError{Msg: err.Error()})
				}
				wait = f
			}
			x.Sleep(wait)
			last = exc
			continue
		}
		var parsedBody any = detail
		if v, err := pyjson.LoadsString(detail); err == nil {
			parsedBody = v
		}
		message := detail
		if o, ok := parsedBody.(*pyjson.Object); ok {
			message = py.Str(o.GetOr("message", detail))
		}
		herr.Raise(&herr.GitHubError{
			Msg:    fmt.Sprintf("%s %s: HTTP %d: %s", method, path, exc.Code, py.SliceTo(redact.Redact(message, nil), 500)),
			Status: exc.Code,
			Body:   parsedBody,
		})
	}
	lastText := "None"
	if last != nil {
		lastText = last.Error()
	}
	herr.Raise(&herr.GitHubError{Msg: fmt.Sprintf("%s %s: %s", method, path, redact.Redact(lastText, nil)), Status: 0})
	return nil
}

// isTransportError is `except (urllib.error.URLError, TimeoutError, ConnectionError)` (an
// HTTPError, a URLError too, is caught before it).
func isTransportError(err error) bool {
	var u *URLError
	var t *TimeoutError
	var c *ConnectionError
	return errors.As(err, &u) || errors.As(err, &t) || errors.As(err, &c)
}

// Paginate is every item of a paged list, up to `limit` (each page a conditional GET if
// `cached`). `key` "" is None: the answer is the list itself.
func (x *GitHub) Paginate(path string, params *pyjson.Object /* = None */, key string /* = None */, limit int /* = 1000 */, cached bool /* = False */) []any {
	query := params.Copy()
	query.SetDefault("per_page", 100)
	items := []any{}
	page := 1
	for len(items) < limit {
		query.Set("page", page)
		chunk := x.Request("GET", path+"?"+urlencode(query), nil, "", false, cached)
		if o, ok := chunk.(*pyjson.Object); ok && key != "" {
			chunk = o.GetOr(key, []any{})
		}
		list, ok := chunk.([]any)
		if !ok || len(list) == 0 {
			break
		}
		items = append(items, list...)
		if len(list) < perPage(query.Get("per_page")) {
			break
		}
		page++
	}
	if len(items) > limit {
		items = items[:max(limit, 0)]
	}
	return items
}

// perPage is int(query["per_page"]).
func perPage(v any) int {
	if s, ok := v.(string); ok {
		n, err := strconv.Atoi(py.Strip(s))
		if err != nil {
			herr.Raise(&ValueError{Msg: "invalid literal for int() with base 10: " + py.Repr(s)})
		}
		return n
	}
	n, _ := pyjson.AsInt(v)
	return n
}

// Graphql is one GraphQL request: its `data`, or a GitHubError (status 200) carrying the errors
// GitHub answered with.
func (x *GitHub) Graphql(query string, variables *pyjson.Object) *pyjson.Object {
	result := x.Request("POST", "/graphql", pyjson.New("query", query, "variables", variables), "", false, false)
	if x.DryRun {
		return pyjson.New()
	}
	o, isDict := result.(*pyjson.Object)
	var errs any
	if isDict {
		errs = o.Get("errors")
	}
	if pyjson.Truthy(errs) {
		var texts []string
		for _, e := range py.Each(errs) {
			if eo, ok := e.(*pyjson.Object); ok {
				texts = append(texts, py.Str(eo.GetOr("message", eo)))
			} else {
				texts = append(texts, py.Str(e))
			}
		}
		text := strings.Join(texts, "; ")
		herr.Raise(&herr.GitHubError{Msg: "graphql: " + py.SliceTo(redact.Redact(text, nil), 500), Status: 200, Body: errs})
	}
	if !isDict {
		return pyjson.New()
	}
	return pyjson.Obj(o.GetOr("data", pyjson.New()))
}

func (x *GitHub) r() string { return "/repos/" + x.Repo }

// objs is a list of GitHub's objects as the typed list the methods return.
func objs(items []any) []*pyjson.Object {
	out := make([]*pyjson.Object, len(items))
	for i, item := range items {
		out[i] = pyjson.Obj(item)
	}
	return out
}

// obj is an answer that should be an object (a nil object, which reads as empty, when it is
// not one).
func obj(v any) *pyjson.Object { return pyjson.Obj(v) }
