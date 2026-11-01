package verdicts

import (
	"os"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata.json holds what Python 3.13 answered (bot/harness/verdicts.py).
func TestMatchesPython(t *testing.T) {
	raw, err := os.ReadFile("testdata.json")
	if err != nil {
		t.Fatal(err)
	}
	v, err := pyjson.Loads(raw)
	if err != nil {
		t.Fatal(err)
	}
	data := pyjson.Obj(v)
	check := func(what string, got, want any) {
		t.Helper()
		if !pyjson.Equal(got, want) {
			t.Errorf("%s = %s, want %s", what, pyjson.Dumps(got), pyjson.Dumps(want))
		}
	}
	for _, row := range pyjson.Objs(data.Get("fixed")) {
		text := row.Str("text")
		for _, tag := range []string{"bot", "review", "suggestions"} {
			value, rest := Header(text, tag)
			check("header("+pyjson.Dumps(text)+", "+tag+")", []any{pyjson.Dumps(value), rest}, row.Get(tag))
		}
		b := BuildReport_(text)
		check("build_report("+pyjson.Dumps(text)+")", pyjson.New("status", b.Status, "title", b.Title,
			"question", b.Question, "body", b.Body, "next_model", b.NextModel, "human_required", b.HumanRequired),
			row.Get("build"))
		r := Review_(text)
		got := r.ToDict()
		var md []any
		for _, f := range r.Findings {
			md = append(md, f.Markdown())
		}
		got.Set("approved", r.Approved()).Set("blocking", len(r.Blocking())).Set("notes", len(r.Notes())).
			Set("md", pyjson.ListOf(md)).Set("why", r.WhyUnreadable())
		check("review("+pyjson.Dumps(text)+")", got, row.Get("review_obj"))
		var sugg []any
		for _, n := range []int{0, 1, 10} {
			var found []any
			for _, s := range Suggestions(text, n) {
				found = append(found, pyjson.New("title", s.Title, "body", s.Body, "kind", s.Kind))
			}
			sugg = append(sugg, pyjson.ListOf(found))
		}
		check("suggestions("+pyjson.Dumps(text)+")", sugg, row.Get("sugg"))
		rating, rest := Rating(text)
		var r0 any
		if rating != nil {
			r0 = pyjson.New("difficulty", rating["difficulty"], "why", rating["why"])
		}
		check("rating("+pyjson.Dumps(text)+")", []any{r0, rest}, row.Get("rating"))
	}
	for _, e := range data.List("fuzz") {
		c := pyjson.List(e)
		value, rest := Header(c[0].(string), "bot")
		check("header("+pyjson.Dumps(c[0])+")", []any{pyjson.Dumps(value), rest}, c[1:])
	}
}
