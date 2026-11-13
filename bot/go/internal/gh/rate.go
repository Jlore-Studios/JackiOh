package gh

import (
	"net/http"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"sync"
	"time"
)

// Rate is what GitHub last said of each token's hourly limits (its `X-RateLimit-*` headers), by
// token and by the limit an answer counted against (`X-RateLimit-Resource`), kept for the process:
// the status loop says how much of the bot's token is left at each rewrite, and rewrites less often
// while it runs low (#3675). REST's limit (`core`) and GraphQL's (`graphql`) are 5,000 an hour
// each, counted apart and reset at different times: kept as one, the loop read GraphQL's for
// REST's and never saw REST's run out until it was spent (#3760). It also counts the calls each
// REST path cost, so the loop can say what spends them. Only what an answer already carried is
// kept; nothing here costs a call.
var Rate = &RateBook{left: map[rateKey]RateLeft{}, spent: map[rateKey]map[string]int{}}

// REST is the `X-RateLimit-Resource` of REST's limit, which every call but GraphQL and search
// counts against; an answer that names none is counted there.
const REST = "core"

// GraphQL is the `X-RateLimit-Resource` of GraphQL's limit, in points.
const GraphQL = "graphql"

// RateLeft is one token's hourly limit as its last answer gave it.
type RateLeft struct {
	Remaining int
	Limit     int
	Reset     time.Time
}

// rateKey is one token's limit for one resource.
type rateKey struct{ token, resource string }

// RateBook is Rate: each token's last RateLeft for each resource, and the calls each path cost,
// behind a lock, since the loop's goroutines share it.
type RateBook struct {
	mu    sync.Mutex
	left  map[rateKey]RateLeft
	spent map[rateKey]map[string]int
}

// resource is the limit `header` says an answer counted against, REST's when it names none.
func resource(header http.Header) string {
	if r := header.Get("X-RateLimit-Resource"); r != "" {
		return r
	}
	return REST
}

// note keeps what `header` says of `token`'s limit, when it says it.
func (b *RateBook) note(token string, header http.Header) {
	if header == nil {
		return
	}
	remaining, err := strconv.Atoi(header.Get("X-RateLimit-Remaining"))
	if err != nil {
		return
	}
	limit, _ := strconv.Atoi(header.Get("X-RateLimit-Limit"))
	var reset time.Time
	if at, err := strconv.ParseInt(header.Get("X-RateLimit-Reset"), 10, 64); err == nil {
		reset = time.Unix(at, 0).UTC()
	}
	b.mu.Lock()
	defer b.mu.Unlock()
	b.left[rateKey{token, resource(header)}] = RateLeft{Remaining: remaining, Limit: limit, Reset: reset}
}

// spend counts an answer to `method` `url` against the limit its header names, as one call of
// its path (`SpendPath`). A 304 costs nothing (a conditional request whose answer had not
// changed), nor does the limit's own refusal once it is spent, and an answer without the headers
// was not counted by GitHub either.
func (b *RateBook) spend(token string, header http.Header, code int, method string, url string) {
	if header == nil || code == 304 || header.Get("X-RateLimit-Remaining") == "" {
		return
	}
	if (code == 403 || code == 429) && header.Get("X-RateLimit-Remaining") == "0" {
		return
	}
	key := rateKey{token, resource(header)}
	b.mu.Lock()
	defer b.mu.Unlock()
	if b.spent[key] == nil {
		b.spent[key] = map[string]int{}
	}
	b.spent[key][SpendPath(method, url)]++
}

// Left is `token`'s REST limit as its last answer gave it, and whether any answer gave it: the one
// GitHub refuses the bot's reads and writes on when it runs out.
func (b *RateBook) Left(token string) (RateLeft, bool) {
	return b.LeftOf(token, REST)
}

// LeftOf is `token`'s limit for `resource` as its last answer gave it, and whether any did.
func (b *RateBook) LeftOf(token string, resource string) (RateLeft, bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	left, ok := b.left[rateKey{token, resource}]
	return left, ok
}

// Take is the calls `token` spent on `resource`'s limit by path since the last Take, and forgets
// them.
func (b *RateBook) Take(token string, resource string) map[string]int {
	b.mu.Lock()
	defer b.mu.Unlock()
	key := rateKey{token, resource}
	spent := b.spent[key]
	delete(b.spent, key)
	if spent == nil {
		spent = map[string]int{}
	}
	return spent
}

// Clear forgets every token's limits and counts (for the tests).
func (b *RateBook) Clear() {
	b.mu.Lock()
	defer b.mu.Unlock()
	b.left = map[rateKey]RateLeft{}
	b.spent = map[rateKey]map[string]int{}
}

// numberOrSHA is a path segment that names one thing among many: a number, or a commit's SHA.
var numberOrSHA = regexp.MustCompile(`^([0-9]+|[0-9a-f]{40})$`)

// SpendPath is how a call is counted: its method and its path under the API, without its query,
// the repository's own `/repos/<owner>/<name>` and with each number or SHA as `{n}`, so the
// calls of one kind count together (`GET /actions/runs/{n}/jobs`).
func SpendPath(method string, url string) string {
	path := strings.TrimPrefix(url, API)
	if i := strings.IndexByte(path, '?'); i >= 0 {
		path = path[:i]
	}
	parts := strings.Split(path, "/")
	if len(parts) >= 4 && parts[0] == "" && parts[1] == "repos" {
		parts = append([]string{""}, parts[4:]...)
	}
	for i, part := range parts {
		if numberOrSHA.MatchString(part) {
			parts[i] = "{n}"
		}
	}
	path = strings.Join(parts, "/")
	if path == "" {
		path = "/"
	}
	return method + " " + path
}

// PathCount is one path's calls (`TopSpent`).
type PathCount struct {
	Path  string
	Calls int
}

// TopSpent is `spent`'s total and its `n` costliest paths, the costliest first, ties by path.
func TopSpent(spent map[string]int, n int) (int, []PathCount) {
	total := 0
	var paths []PathCount
	for path, calls := range spent {
		total += calls
		paths = append(paths, PathCount{path, calls})
	}
	sort.Slice(paths, func(i, j int) bool {
		if paths[i].Calls != paths[j].Calls {
			return paths[i].Calls > paths[j].Calls
		}
		return paths[i].Path < paths[j].Path
	})
	if len(paths) > n {
		paths = paths[:n]
	}
	return total, paths
}
