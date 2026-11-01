package cardsets

import (
	"net/url"
	"os"
	"reflect"
	"strings"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json is what bot/harness/cardsets.py read and drew for tests/test_cardsets.py's
// fake production (written by a script run from bot/ against the Python harness).
func golden(t *testing.T) *pyjson.Object {
	t.Helper()
	data, err := os.ReadFile("testdata/python.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(data)
	if err != nil {
		t.Fatal(err)
	}
	return v.(*pyjson.Object)
}

// production is tests/test_cardsets.py's Production: a fake `/api/stats/cards` over a history of
// [version, live, dev], recording what it was asked.
type production struct {
	by     map[string][2]any
	newest string
	asked  [][2]string
}

func newProduction(history []any) *production {
	p := &production{by: map[string][2]any{}}
	for i, h := range history {
		entry := pyjson.List(h)
		version := pyjson.Str(entry[0])
		if i == 0 {
			p.newest = version
		}
		p.by[version] = [2]any{entry[1], entry[2]}
	}
	return p
}

func (p *production) fetch(raw string) any {
	parsed, err := url.Parse(raw)
	if err != nil {
		panic(err)
	}
	query := parsed.Query()
	version, source := p.newest, query.Get("source")
	if query.Has("patch") {
		version = query.Get("patch")
	}
	p.asked = append(p.asked, [2]string{version, source})
	live, dev := p.by[version][0], p.by[version][1]
	if source == "dev" && dev != nil {
		return dev
	}
	return live
}

func floatOrNil(v *float64) any {
	if v == nil {
		return nil
	}
	return *v
}

func TestReadWalksBackToTheLastThreePatches(t *testing.T) {
	g := golden(t)
	api := newProduction(g.List("history"))
	patches, minSample, oldest := Read(api.fetch, API)
	var read []any
	for _, p := range patches {
		read = append(read, []any{p.Name, p.Live, p.Games})
	}
	if !pyjson.Equal(read, g.Get("read")) || minSample != g.Int("min_sample") || oldest != g.Str("oldest") {
		t.Fatalf("read %v, %d, %q", read, minSample, oldest)
	}
	var asked []any
	for _, a := range api.asked {
		asked = append(asked, []any{a[0], a[1]})
	}
	if !pyjson.Equal(asked, g.Get("asked")) {
		t.Fatalf("asked %v", asked)
	}
	for i, row := range Rows(patches, minSample) {
		want := pyjson.Objs(g.Get("rows"))[i]
		got := pyjson.New("name", row.Name, "cards", row.Cards, "games", row.Games, "wins", row.Wins,
			"rates", pyjson.ListOf(row.Rates), "by_patch", row.ByPatch, "pooled", floatOrNil(row.Pooled()),
			"mean", floatOrNil(row.Mean()), "median", floatOrNil(row.Median()), "error", floatOrNil(row.Error()))
		if pyjson.Dumps(got) != pyjson.Dumps(want) {
			t.Errorf("row %d:\n got %s\nwant %s", i, pyjson.Dumps(got), pyjson.Dumps(want))
		}
	}
}

func TestTheSectionIsDrawnAsPythonDrewIt(t *testing.T) {
	g := golden(t)
	when := g.Str("when")
	for _, c := range []struct{ history, drawn string }{{"history", "draw"}, {"many_history", "draw_many"},
		{"thin_history", "draw_thin"}} {
		patches, minSample, oldest := Read(newProduction(g.List(c.history)).fetch, API)
		got, want := draw(when, patches, minSample, oldest), g.Str(c.drawn)
		if got != want {
			gl, wl := strings.Split(got, "\n"), strings.Split(want, "\n")
			for i := range max(len(gl), len(wl)) {
				if i >= len(gl) || i >= len(wl) || gl[i] != wl[i] {
					t.Fatalf("%s differs at line %d:\n got %q\nwant %q", c.drawn, i+1, gl[min(i, len(gl)-1)],
						wl[min(i, len(wl)-1)])
				}
			}
		}
	}
}

func TestMeanAndMedianAreStatisticsOwn(t *testing.T) {
	g := golden(t)
	for _, c := range pyjson.Objs(g.Get("fsums")) {
		var xs []float64
		for _, x := range c.List("xs") {
			f, _ := pyjson.AsFloat(x)
			xs = append(xs, f)
		}
		row := &SetRow{Rates: xs}
		if got := fsum(xs); got != c.Float("fsum") {
			t.Errorf("fsum of %d: got %v, want %v", len(xs), got, c.Float("fsum"))
		}
		if got := *row.Mean(); got != c.Float("mean") {
			t.Errorf("mean of %d: got %v, want %v", len(xs), got, c.Float("mean"))
		}
		if got := *row.Median(); got != c.Float("median") {
			t.Errorf("median of %d: got %v, want %v", len(xs), got, c.Float("median"))
		}
	}
	if (&SetRow{}).Mean() != nil || (&SetRow{}).Median() != nil || (&SetRow{}).Pooled() != nil || (&SetRow{}).Error() != nil {
		t.Error("an empty set has no rates")
	}
}

func TestURLsAreUrlencoded(t *testing.T) {
	g := golden(t)
	got := []string{URL("https://x", "", "live"), URL("https://x", "v0.4.0b", "dev"), URL("https://x", "a b&c/é", "live")}
	if want := pyjson.Strings(g.Get("urls")); !reflect.DeepEqual(got, want) {
		t.Fatalf("got %q, want %q", got, want)
	}
}

func TestAMalformedReadRaisesWhatPythonRaised(t *testing.T) {
	read := func(payload string) (*ReadError, []*Patch) {
		v, err := pyjson.LoadsString(payload)
		if err != nil {
			t.Fatal(err)
		}
		var patches []*Patch
		exc, _ := herr.Catch[*ReadError](func() { patches, _, _ = Read(func(string) any { return v }, API) })
		return exc, patches
	}
	for payload, want := range map[string][2]string{
		`[1]`: {"AttributeError", "'list' object has no attribute 'get'"},
		`{"patch": "v1.0.0", "gate": {"cleared": true}, "totalGames": 5, "cards": [{"games": 1}]}`: {"KeyError", "'id'"},
		`{"patch": "v1.0.0", "gate": {"cleared": true}, "cards": [{"id": "a", "games": "abc"}]}`: {"ValueError",
			"invalid literal for int() with base 10: 'abc'"},
		`{"patch": "v1.0.0", "gate": {"cleared": true}, "cards": [{"id": "a", "games": 2, "winRate": [1]}]}`: {"TypeError",
			"float() argument must be a string or a real number, not 'list'"},
		`{"patch": "v1.0.0", "gate": {"cleared": true}, "cards": 5}`: {"TypeError", "'int' object is not iterable"},
		`{"patch": "v1.0.0", "gate": "yes"}`:                         {"AttributeError", "'str' object has no attribute 'get'"},
	} {
		exc, _ := read(payload)
		if exc == nil || exc.Type != want[0] || exc.Msg != want[1] {
			t.Errorf("%s: got %+v, want %v", payload, exc, want)
		}
	}
	exc, patches := read(`{"patch": "v1.0.0", "gate": {"cleared": true}, "totalGames": " 1_000 ",
		"cards": [{"id": "a", "games": "4", "winRate": "0.5"}]}`)
	if exc != nil || len(patches) != 1 || patches[0].Games != 1000 {
		t.Fatalf("int() and float() read numbers in strings: %+v %v", exc, patches)
	}
	if card := patches[0].Cards.Get("a").(*Card); card.Wins != 2 || card.Games != 4 {
		t.Fatalf("card %+v", card)
	}
}

func TestKeptFindsTheSection(t *testing.T) {
	text := strings.Join([]string{Start, Heading, "", End}, "\n")
	if Kept("x\n"+text+"\ny") != text || Kept("nothing") != "" {
		t.Fatal("Kept")
	}
	failed := Failed.ReplaceAllString(Heading+"\n\n> ⚠️ The read failed.\n\nrest", "")
	if failed != Heading+"\n\nrest" {
		t.Fatalf("FAILED.sub: %q", failed)
	}
}
