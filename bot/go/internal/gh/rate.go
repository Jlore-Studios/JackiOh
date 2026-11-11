package gh

import (
	"net/http"
	"strconv"
	"sync"
	"time"
)

// Rate is what GitHub last said of each token's hourly limit (its `X-RateLimit-*` headers), by
// token, kept for the process: the status loop says how much of the bot's token is left at each
// rewrite, and rewrites less often while it runs low (#3675). Only what an answer already carried
// is kept; nothing here costs a call.
var Rate = &RateBook{left: map[string]RateLeft{}}

// RateLeft is one token's hourly limit as its last answer gave it.
type RateLeft struct {
	Remaining int
	Limit     int
	Reset     time.Time
}

// RateBook is Rate: each token's last RateLeft, behind a lock, since the loop's goroutines share
// it.
type RateBook struct {
	mu   sync.Mutex
	left map[string]RateLeft
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
	b.left[token] = RateLeft{Remaining: remaining, Limit: limit, Reset: reset}
}

// Left is `token`'s limit as its last answer gave it, and whether any answer gave it.
func (b *RateBook) Left(token string) (RateLeft, bool) {
	b.mu.Lock()
	defer b.mu.Unlock()
	left, ok := b.left[token]
	return left, ok
}

// Clear forgets every token's limit (for the tests).
func (b *RateBook) Clear() {
	b.mu.Lock()
	defer b.mu.Unlock()
	b.left = map[string]RateLeft{}
}
