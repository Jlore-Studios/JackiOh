package triage

import (
	"os"
	"path/filepath"
	"reflect"
	"testing"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// testdata/python.json holds the Python's answers for the same inputs (harness.triage, run with
// Python 3.13).
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

func strs(v any) []string {
	out := []string{}
	for _, s := range pyjson.List(v) {
		out = append(out, s.(string))
	}
	return out
}

func TestTitleChecksAsPython(t *testing.T) {
	for _, c := range python(t, "pull_title_ok") {
		row := pyjson.List(c)
		if got := PullTitleOK(row[0].(string)); got != row[1] {
			t.Errorf("PullTitleOK(%q) = %v; Python %v", row[0], got, row[1])
		}
	}
	for _, c := range python(t, "pull_title") {
		row := pyjson.List(c)
		if got := PullTitle(row[0].(string), row[1].(string)); got != row[2] {
			t.Errorf("PullTitle(%q, %q) = %q; Python %q", row[0], row[1], got, row[2])
		}
	}
	for _, c := range python(t, "type_label") {
		row := pyjson.List(c)
		if got := TypeLabel(row[0].(string)); got != row[1] {
			t.Errorf("TypeLabel(%q) = %q; Python %q", row[0], got, row[1])
		}
	}
}

// The form's answers without the lookahead RE2 lacks, against `_FORM_ANSWER.finditer`.
func TestFormAnswersAsPython(t *testing.T) {
	for _, c := range python(t, "form_answers") {
		row := pyjson.List(c)
		want := [][2]string{}
		for _, m := range pyjson.List(row[1]) {
			pair := strs(m)
			want = append(want, [2]string{pair[0], pair[1]})
		}
		got := formAnswers(replaceCRLF(row[0].(string)))
		if got == nil {
			got = [][2]string{}
		}
		if !reflect.DeepEqual(got, want) {
			t.Errorf("formAnswers(%q) = %q; Python %q", row[0], got, want)
		}
	}
}

func replaceCRLF(s string) string {
	out := []rune{}
	rs := []rune(s)
	for i := 0; i < len(rs); i++ {
		if rs[i] == '\r' && i+1 < len(rs) && rs[i+1] == '\n' {
			continue
		}
		out = append(out, rs[i])
	}
	return string(out)
}

func TestFormLabelsAsPython(t *testing.T) {
	for _, c := range python(t, "form_labels") {
		row := pyjson.List(c)
		if got := FormLabels(row[0].(string)); !reflect.DeepEqual(got, strs(row[1])) {
			t.Errorf("FormLabels(%q) = %q; Python %q", row[0], got, strs(row[1]))
		}
	}
}

func TestFallbackTypeAsPython(t *testing.T) {
	for _, c := range python(t, "fallback_type") {
		row := pyjson.List(c)
		issue := pyjson.New("title", row[0], "labels", row[1])
		if got := FallbackType(issue, DefaultIssueTypes); got != row[2] {
			t.Errorf("FallbackType(%q, %v) = %q; Python %q", row[0], row[1], got, row[2])
		}
	}
	if got := FallbackType(pyjson.New("title", "It crashes"), map[string]string{"Task": ""}); got != "" {
		t.Errorf("an organisation without Bug types nothing; got %q", got)
	}
}

func thread(title string, labels []string, assignees []string) *pyjson.Object {
	ls, as := []any{}, []any{}
	for _, l := range labels {
		ls = append(ls, pyjson.New("name", l))
	}
	for _, a := range assignees {
		as = append(as, pyjson.New("login", a))
	}
	return pyjson.New("number", 40, "title", title, "body", "Please fix it.", "labels", ls,
		"assignees", as, "state", "open", "type", nil)
}

// test_triage.py's test_human_moves_it_to_people_and_off_the_bots_queue, without the queue
// labels (config's, a stub in this worktree).
func TestHumanMovesItToPeople(t *testing.T) {
	plan := Decide("human", thread("x", []string{"human", "bot:working", "patch"}, []string{"jgoetzmann-bot", "MaxGoetzmann"}),
		false, "jgoetzmann-bot", nil, true)
	if !reflect.DeepEqual(plan.Assignees, []string{"jgoetzmann"}) || !reflect.DeepEqual(plan.Unassign, []string{"jgoetzmann-bot"}) {
		t.Fatalf("plan = %+v", plan)
	}
	gone := Decide("human", thread("x", []string{"patch"}, nil), false, "jgoetzmann-bot", nil, true)
	if !gone.Empty() || gone.Notes[0] != "`human` came off again, so it stays as it is" {
		t.Fatalf("plan = %+v", gone)
	}
	closed := thread("Patch v0.3.Y: x", nil, nil)
	closed.Set("state", "closed")
	if plan := Decide("open", closed, false, "jgoetzmann-bot", nil, true); !plan.Empty() || plan.Notes[0] != "closed meanwhile" {
		t.Fatalf("plan = %+v", plan)
	}
}

func TestAnIssueGetsATypeAndAPersonsTypeStays(t *testing.T) {
	for title, want := range map[string]string{
		"The board crashes on a trap": "Bug", "Night bot: a thing": "Task", "A new leaderboard page": "Feature",
	} {
		if got := Decide("open", thread(title, nil, nil), false, "b", nil, true).IssueType; got != want {
			t.Errorf("%q typed %q; want %q", title, got, want)
		}
	}
	typed := thread("It crashes", nil, nil)
	typed.Set("type", pyjson.New("name", "Task"))
	if got := Decide("open", typed, false, "b", nil, true).IssueType; got != "" {
		t.Errorf("a person's type stays; got %q", got)
	}
	if got := Decide("open", thread("It crashes", nil, nil), true, "b", nil, true).IssueType; got != "" {
		t.Errorf("a pull request has no type; got %q", got)
	}
}

func TestThreadOf(t *testing.T) {
	issue := pyjson.New("number", 5)
	got, isPR := ThreadOf(pyjson.New("issue", issue))
	if isPR || got.Int("number") != 5 || got == issue {
		t.Fatal("an issue's thread is a copy of it")
	}
	if _, isPR := ThreadOf(pyjson.New("issue", pyjson.New("pull_request", pyjson.New()))); !isPR {
		t.Fatal("an issue event about a pull request")
	}
	if got, isPR := ThreadOf(pyjson.New("pull_request", pyjson.New("number", 6))); !isPR || got.Int("number") != 6 {
		t.Fatal("a pull request event")
	}
}
