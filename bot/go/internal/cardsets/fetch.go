package cardsets

import (
	"context"
	"errors"
	"fmt"
	"io"
	"net"
	"net/http"
	"strconv"
	"strings"
	"syscall"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// FetchJSON is `url`'s JSON, the way the status loop reads production: a GET with
// `Accept: application/json` and `User-Agent: jackioh-bot`, through urllib's proxies, as
// HTTP/1.1 with `Connection: close`, every socket operation (connect, TLS, each read and write)
// given `TIMEOUT_SECONDS`. What urlopen and json.load raised it raises as a *ReadError of the same
// name (`HTTPError` for a status outside 2xx, `URLError` when the request could not be sent,
// `TimeoutError` when the answer stalled, `JSONDecodeError` when it is not JSON); Go's network
// errors word their reasons differently from Python's.
func FetchJSON(url string) any {
	request, err := http.NewRequest("GET", url, nil)
	if err != nil {
		raise("ValueError", "unknown url type: "+py.Repr(url))
	}
	request.Header.Set("Accept", "application/json")
	request.Header.Set("User-Agent", "jackioh-bot")
	request.Header.Set("Accept-Encoding", "identity")
	request.Close = true
	response, err := client().Do(request)
	if err != nil {
		raiseNetwork(err)
	}
	defer response.Body.Close()
	if response.StatusCode < 200 || response.StatusCode > 299 {
		reason := strings.TrimPrefix(response.Status, strconv.Itoa(response.StatusCode)+" ")
		raise("HTTPError", fmt.Sprintf("HTTP Error %d: %s", response.StatusCode, reason))
	}
	data, err := io.ReadAll(response.Body)
	if err != nil {
		raiseNetwork(err)
	}
	value, err := pyjson.Loads(data)
	if err != nil {
		raise("JSONDecodeError", err.Error())
	}
	return value
}

// timeoutConn is a socket with Python's timeout: each read and write may take TIMEOUT_SECONDS.
type timeoutConn struct {
	net.Conn
	timeout time.Duration
}

func (c *timeoutConn) Read(b []byte) (int, error) {
	if err := c.Conn.SetReadDeadline(time.Now().Add(c.timeout)); err != nil {
		return 0, err
	}
	return c.Conn.Read(b)
}

func (c *timeoutConn) Write(b []byte) (int, error) {
	if err := c.Conn.SetWriteDeadline(time.Now().Add(c.timeout)); err != nil {
		return 0, err
	}
	return c.Conn.Write(b)
}

func client() *http.Client {
	timeout := time.Duration(TimeoutSeconds) * time.Second
	dialer := &net.Dialer{Timeout: timeout}
	return &http.Client{Transport: &http.Transport{
		Proxy: http.ProxyFromEnvironment,
		DialContext: func(ctx context.Context, network, addr string) (net.Conn, error) {
			conn, err := dialer.DialContext(ctx, network, addr)
			if err != nil {
				return nil, err
			}
			return &timeoutConn{conn, timeout}, nil
		},
		DisableKeepAlives: true,
	}}
}

// raiseNetwork raises a failed request as urllib did: URLError while connecting or sending (an
// OSError in `h.request`), and the OSError itself once the request went out (a timeout waiting for
// the answer is `TimeoutError: The read operation timed out`).
func raiseNetwork(err error) {
	var op *net.OpError
	sending := errors.As(err, &op) && (op.Op == "dial" || op.Op == "proxyconnect" || op.Op == "write")
	var timeout interface{ Timeout() bool }
	timedOut := errors.As(err, &timeout) && timeout.Timeout()
	var dns *net.DNSError
	var errno syscall.Errno
	reason := ""
	switch {
	case timedOut && sending:
		reason = "timed out"
	case timedOut:
		raise("TimeoutError", "The read operation timed out")
	case errors.As(err, &dns) && dns.IsNotFound:
		reason = "[Errno -2] Name or service not known"
	case errors.As(err, &dns):
		reason = "[Errno -3] Temporary failure in name resolution"
	case errors.As(err, &errno):
		reason = py.ErrnoText(errno)
		if !sending {
			raise(py.OSErrorName(errno), reason)
		}
	default:
		inner := err
		for errors.Unwrap(inner) != nil {
			inner = errors.Unwrap(inner)
		}
		reason = inner.Error()
	}
	raise("URLError", "<urlopen error "+reason+">")
}
