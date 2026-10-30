package gh

// What urllib does for gh.py, which Go's net/http does otherwise: the opener a client sends its
// requests through (`urllib.request.urlopen`, and `build_opener(_NoRedirect)` for the two calls
// whose redirect must not carry the token), the errors it raises, and how it decodes a body.

import (
	"bytes"
	"context"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"net/http/httptrace"
	"strconv"
	"strings"
	"sync/atomic"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// Request is urllib.request.Request as the client builds it: the method, the URL, the headers
// and the body (nil for none).
type Request struct {
	Method string
	URL    string
	Header http.Header
	Body   []byte
}

// Response is what an opener answers with for a 2xx status: its headers and its body, read
// whole.
type Response struct {
	Header http.Header
	Body   []byte
}

// Opener is the shape of `urllib.request.urlopen(req, timeout=…)`: the answer to a 2xx status,
// an *HTTPError for any other status (a redirect the opener does not follow included), and a
// *URLError, *TimeoutError or *ConnectionError when no answer arrived.
type Opener func(req *Request, timeout time.Duration) (*Response, error)

// HTTPError is urllib.error.HTTPError: the server answered with a status the opener does not
// take as success. Body is nil when the answer had none (Python's `exc.fp` None).
type HTTPError struct {
	URL    string
	Code   int
	Msg    string
	Header http.Header
	Body   []byte
}

func (e *HTTPError) Error() string { return fmt.Sprintf("HTTP Error %d: %s", e.Code, e.Msg) }

// URLError is urllib.error.URLError: the request could not be sent (no connection, a name that
// does not resolve, a refused TLS handshake). Reason is the text of the OSError it wraps.
type URLError struct{ Reason string }

func (e *URLError) Error() string { return "<urlopen error " + e.Reason + ">" }

// TimeoutError is Python's TimeoutError: a read on a connection that was made waited longer
// than the timeout.
type TimeoutError struct{}

func (e *TimeoutError) Error() string { return "timed out" }

// ConnectionError is Python's ConnectionError: a connection that was made broke before the
// answer was read.
type ConnectionError struct{ Msg string }

func (e *ConnectionError) Error() string { return e.Msg }

// Urlopen is urllib.request.urlopen: a client's opener when it is given none, and the fetch of
// the storage URL a log or an artifact redirects to. Redirects are followed as urllib follows
// them. A variable so the tests can replace it.
var Urlopen Opener = func(req *Request, timeout time.Duration) (*Response, error) {
	return open(req, timeout, true)
}

// BuildOpener is `urllib.request.build_opener(_NoRedirect)`: an opener that answers a redirect
// with an *HTTPError, so the location is fetched without the token. A variable so the tests can
// replace it.
var BuildOpener = func() Opener {
	return func(req *Request, timeout time.Duration) (*Response, error) {
		return open(req, timeout, false)
	}
}

// redirectCodes are the statuses urllib's HTTPRedirectHandler follows.
var redirectCodes = map[int]bool{301: true, 302: true, 303: true, 307: true, 308: true}

// open sends one request as urllib does: HTTP/1.1, a new connection closed after the answer
// (`Connection: close`), `Accept-Encoding: identity`, the proxy the environment names, and the
// timeout on every blocking operation of the socket (connect, each read and write) rather than
// on the whole exchange. `follow` follows redirects as HTTPRedirectHandler does; without it a
// redirect is an HTTPError (`_NoRedirect`).
func open(req *Request, timeout time.Duration, follow bool) (*Response, error) {
	var body io.Reader
	if req.Body != nil {
		body = bytes.NewReader(req.Body)
	}
	var connected atomic.Bool
	trace := &httptrace.ClientTrace{GotConn: func(httptrace.GotConnInfo) { connected.Store(true) }}
	hr, err := http.NewRequestWithContext(httptrace.WithClientTrace(context.Background(), trace),
		req.Method, req.URL, body)
	if err != nil {
		return nil, &URLError{Reason: err.Error()}
	}
	for k, vs := range req.Header {
		hr.Header[k] = append([]string(nil), vs...)
	}
	hr.Header.Set("Accept-Encoding", "identity")
	client := &http.Client{
		Transport: transport(timeout),
		CheckRedirect: func(next *http.Request, via []*http.Request) error {
			connected.Store(false)
			if !follow {
				return http.ErrUseLastResponse
			}
			return redirectAllowed(next, via)
		},
	}
	resp, err := client.Do(hr)
	if err != nil {
		return nil, classify(err, connected.Load())
	}
	defer resp.Body.Close()
	data, err := io.ReadAll(resp.Body)
	if err != nil {
		return nil, classify(err, true)
	}
	if resp.StatusCode < 200 || resp.StatusCode >= 300 {
		return nil, &HTTPError{
			URL:    resp.Request.URL.String(),
			Code:   resp.StatusCode,
			Msg:    strings.TrimPrefix(strings.TrimPrefix(resp.Status, strconv.Itoa(resp.StatusCode)), " "),
			Header: resp.Header,
			Body:   data,
		}
	}
	return &Response{Header: resp.Header, Body: data}, nil
}

// redirectAllowed is HTTPRedirectHandler's rule: a GET or HEAD follows any redirect, a POST
// follows 301, 302 and 303 as a GET, and any other is answered as the HTTPError it is; a URL
// seen four times, or ten URLs, end it the same way (`max_repeats`, `max_redirections`). The new
// request carries every header of the first but Content-Length and Content-Type, Authorization
// included whatever its host (Go drops that for another host; urllib does not).
func redirectAllowed(next *http.Request, via []*http.Request) error {
	code := next.Response.StatusCode
	method := via[len(via)-1].Method
	if !(redirectCodes[code] && (method == "GET" || method == "HEAD")) &&
		!((code == 301 || code == 302 || code == 303) && method == "POST") {
		return http.ErrUseLastResponse
	}
	visited := map[string]int{}
	for _, r := range via[1:] {
		visited[r.URL.String()]++
	}
	if visited[next.URL.String()] >= 4 || len(visited) >= 10 {
		return http.ErrUseLastResponse
	}
	for k, vs := range via[0].Header {
		if k != "Content-Length" && k != "Content-Type" {
			next.Header[k] = append([]string(nil), vs...)
		}
	}
	return nil
}

func transport(timeout time.Duration) *http.Transport {
	dialer := &net.Dialer{Timeout: timeout}
	return &http.Transport{
		Proxy: http.ProxyFromEnvironment,
		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
			conn, err := dialer.DialContext(ctx, network, addr)
			if err != nil {
				return nil, err
			}
			return &deadlineConn{Conn: conn, timeout: timeout}, nil
		},
		DisableKeepAlives:  true,
		DisableCompression: true,
	}
}

// deadlineConn is a socket with Python's timeout: each read and write may wait that long.
type deadlineConn struct {
	net.Conn
	timeout time.Duration
}

func (c *deadlineConn) Read(b []byte) (int, error) {
	_ = c.Conn.SetReadDeadline(time.Now().Add(c.timeout))
	return c.Conn.Read(b)
}

func (c *deadlineConn) Write(b []byte) (int, error) {
	_ = c.Conn.SetWriteDeadline(time.Now().Add(c.timeout))
	return c.Conn.Write(b)
}

// classify turns a failed exchange into the exception urllib raises for it: a URLError before
// the connection was made (urlopen wraps every OSError of sending), and after it a TimeoutError
// or a ConnectionError (reading the answer is outside that wrap).
func classify(err error, connected bool) error {
	if !connected {
		return &URLError{Reason: osErrorText(err)}
	}
	var ne net.Error
	if errors.As(err, &ne) && ne.Timeout() {
		return &TimeoutError{}
	}
	if errors.Is(err, io.EOF) || errors.Is(err, io.ErrUnexpectedEOF) {
		return &ConnectionError{Msg: "Remote end closed connection without response"}
	}
	return &ConnectionError{Msg: osErrorText(err)}
}

// osErrorText is str(OSError) for the errors a socket raises, as near as Go's errors allow:
// "timed out", "[Errno 111] Connection refused", a name that does not resolve as getaddrinfo
// says it; anything else in Go's words.
func osErrorText(err error) string {
	var ne net.Error
	if errors.As(err, &ne) && ne.Timeout() {
		return "timed out"
	}
	var dns *net.DNSError
	if errors.As(err, &dns) {
		if dns.IsNotFound {
			return "[Errno -2] Name or service not known"
		}
		return "[Errno -3] Temporary failure in name resolution"
	}
	var errno syscall.Errno
	if errors.As(err, &errno) {
		return py.ErrnoText(errno)
	}
	return err.Error()
}
