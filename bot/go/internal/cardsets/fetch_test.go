package cardsets

import (
	"net/http"
	"net/http/httptest"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

func TestFetchJSONReadsAsUrllibDid(t *testing.T) {
	var headers http.Header
	server := httptest.NewServer(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		headers = r.Header.Clone()
		switch r.URL.Path {
		case "/ok":
			_, _ = w.Write([]byte(`{"patch": "v0.4.0", "totalGames": 3}`))
		case "/down":
			w.WriteHeader(http.StatusServiceUnavailable)
		default:
			_, _ = w.Write([]byte(`<html>`))
		}
	}))
	defer server.Close()

	got := FetchJSON(server.URL + "/ok")
	if !pyjson.Equal(got, pyjson.New("patch", "v0.4.0", "totalGames", 3)) {
		t.Fatalf("got %v", got)
	}
	if headers.Get("Accept") != "application/json" || headers.Get("User-Agent") != "jackioh-bot" ||
		headers.Get("Accept-Encoding") != "identity" {
		t.Fatalf("headers %v", headers)
	}
	for path, want := range map[string][2]string{"/down": {"HTTPError", "HTTP Error 503: Service Unavailable"},
		"/html": {"JSONDecodeError", ""}} {
		exc, ok := herr.Catch[*ReadError](func() { FetchJSON(server.URL + path) })
		if !ok || exc.Type != want[0] || (want[1] != "" && exc.Msg != want[1]) {
			t.Errorf("%s: got %+v", path, exc)
		}
	}
	exc, ok := herr.Catch[*ReadError](func() { FetchJSON("http://127.0.0.1:1/") })
	if !ok || exc.Type != "URLError" || exc.Msg != "<urlopen error [Errno 111] Connection refused>" {
		t.Errorf("refused: got %+v", exc)
	}
}
