// Package cardsets is bot/harness/cardsets.py.
//
// Which set's cards win most, in the status issue's statistics (#148): each set's card win rates
// over the last three patches with 1,000 games or more, read once a day from production's public card
// statistics (`GET /api/stats/cards`, SPEC §9.11) and kept in the issue between reads.
//
//   - **A patch** is a patch or major version with its micros (`v0.4.0`, `v0.4.0b`, …): the window
//     every card shares (R1367000). Its figures are the endpoint's at its newest version, found by
//     walking back from production's newest version through `previousPatch`.
//   - **Its games** are its live games once it has the gate's 1,000 of them (the endpoint then counts
//     nothing else, R654), else its live and AI development games together (`?source=live` plus
//     `?source=dev`, the provisional figures the gate pads with). A patch counts with `MIN_GAMES`.
//   - **A card's win rate** is R377's in-deck rate: its deck's wins over the games it was in a deck.
//     A set's pooled rate is its cards' wins over their games; its mean and median are over the cards
//     with the endpoint's `minSample` games or more. Decks mix sets (SPEC §2), so a set's rate says how
//     its cards fare in mixed decks, not how a deck of it would.
//
// A count of the statistics reads the figures afresh once `EVERY` has passed since the last good read,
// or `RETRY` after a failed one; a failed read keeps the last good section and says when it failed.
// Only the status loop reads production (`fetch_json`); with no fetcher, as in a test, the section
// the issue holds is kept as it is.
package cardsets

import (
	"fmt"
	"math"
	"net/url"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/clock"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

var (
	Start   = "<!-- jackioh-bot:card-sets -->"
	End     = "<!-- /jackioh-bot:card-sets -->"
	Heading = "### 🃏 Which set's cards win most"
)

// API is production's server (`render.yaml`), whose public card statistics are read.
var API = "https://jackioh-server.onrender.com"

// How often the figures are read, and how soon a failed read is tried again.
var (
	Every = 24 * time.Hour
	Retry = time.Hour
)

// How many patches are read, and the games a patch needs to be one of them.
var (
	Patches  = 3
	MinGames = 1000
)

// Walk is the most versions, micros included, the walk back reads, so a long run of thin patches
// costs a bounded number of calls.
var Walk = 30

// TimeoutSeconds: a sleeping Render instance takes up to a minute to wake.
var TimeoutSeconds = 120

// OutlierZ: a card this many standard errors from 50% is listed.
var OutlierZ = 3.0

// OutliersShown is how many of them the section lists, furthest first: the status issue keeps under
// GitHub's limit on its body (`dashboard.MAX_BODY`), and the rest are in `/api/stats/cards`.
var OutliersShown = 20

// MinSample is the endpoint's sample floor (`CARD_STATS_MIN_SAMPLE`), when a response does not say
// it.
var MinSample = 20

// ErrorShown is how much of a failed read's error the section repeats.
var ErrorShown = 160

// Version: `v0.4.0b` belongs to the patch `v0.4.0`. (Python's `$` also matches before a final
// newline, hence the `\n?`.)
var Version = regexp.MustCompile(`^(v\d+\.\d+\.\d+)[a-z]*\n?$`)

var Failed = regexp.MustCompile(`(?m)^> ⚠️ .*\n\n`)

// Fetch reads a URL's JSON. It raises (panics with) a *ReadError for what `section` catches.
type Fetch func(url string) any

// ReadError is one of the Python exceptions `section` catches from a read and names in its note
// (OSError, ValueError, LookupError, TypeError, AttributeError and their subclasses): Type is
// type(exc).__name__ (`URLError`, `HTTPError`, `TimeoutError`, `JSONDecodeError`, `KeyError`, …)
// and Msg is str(exc). It is py's Error, so what py's int(), float() and iteration raise in a read
// is one too.
type ReadError = py.Error

func raise(typ, msg string) { herr.Raise(&ReadError{Type: typ, Msg: msg}) }

// Card is cardsets.Card.
type Card struct {
	ID    string
	Name  string
	Set   string
	Wins  int
	Games int
}

// Patch is cardsets.Patch. Build it with NewPatch.
type Patch struct {
	// `v0.4.0`, and the newest of its versions production has shipped, read for its figures.
	Name    string
	Version string
	// Whether it has the gate's live games, so its figures are live games alone.
	Live  bool
	Games int
	// Card id -> *Card, in the order the endpoint gave them.
	Cards *pyjson.Object
}

// NewPatch is Patch(name, version, live, games).
func NewPatch(name, version string, live bool, games int) *Patch {
	return &Patch{Name: name, Version: version, Live: live, Games: games, Cards: pyjson.New()}
}

func cardsOf(cards *pyjson.Object) []*Card {
	var out []*Card
	for _, id := range cards.Keys() {
		out = append(out, cards.Get(id).(*Card))
	}
	return out
}

// SetRow is cardsets.SetRow.
type SetRow struct {
	Name  string
	Cards int
	Games int
	Wins  int
	// The win rate of each of its cards with the sample floor's games.
	Rates []float64
	// Patch name -> its pooled rate (float64), or nil when it has no games of the set.
	ByPatch *pyjson.Object
}

func (x *SetRow) Pooled() *float64 {
	if x.Games != 0 {
		p := float64(x.Wins) / float64(x.Games)
		return &p
	}
	return nil
}

func (x *SetRow) Mean() *float64 {
	if len(x.Rates) > 0 {
		m := fsum(x.Rates) / float64(len(x.Rates))
		return &m
	}
	return nil
}

func (x *SetRow) Median() *float64 {
	if len(x.Rates) > 0 {
		data := append([]float64{}, x.Rates...)
		sort.Float64s(data)
		n := len(data)
		m := data[n/2]
		if n%2 == 0 {
			m = (data[n/2-1] + data[n/2]) / 2
		}
		return &m
	}
	return nil
}

// Error is the pooled rate's standard error, as if every card-game were independent: they are not
// (one game counts every card of both decks), so it is a floor.
func (x *SetRow) Error() *float64 {
	p := x.Pooled()
	if p == nil {
		return nil
	}
	e := sqrt(*p * (1 - *p) / float64(x.Games))
	return &e
}

// URL is cardsets._url.
func URL(base string, version string, source string) string {
	query := "source=" + url.QueryEscape(source)
	if version != "" {
		query += "&patch=" + url.QueryEscape(version)
	}
	return base + "/api/stats/cards?" + query
}

func add(cards *pyjson.Object, rows any) {
	for _, row := range py.MustIter(py.Or(rows, []any{})) {
		games := py.MustInt(py.Or(get(row, "games"), 0))
		rate := get(row, "winRate")
		id := py.Str(item(row, "id"))
		name := get(row, "name")
		if !pyjson.Truthy(name) {
			name = item(row, "id")
		}
		fresh := &Card{ID: py.Str(item(row, "id")), Name: py.Str(name), Set: py.Str(py.Or(get(row, "set"), "?"))}
		card, ok := cards.Get(id).(*Card)
		if !ok {
			card = fresh
			cards.Set(id, card)
		}
		card.Games += games
		if rate != nil {
			card.Wins += py.Round(py.MustFloat(rate) * float64(games))
		}
	}
}

// Read is the last `PATCHES` patches with `MIN_GAMES` games or more, newest first; the endpoint's
// sample floor; and the oldest version the walk read.
func Read(fetch Fetch, base string /* = API */) ([]*Patch, int, string) {
	var found []*Patch
	seen := map[string]bool{}
	version := ""
	minSample := MinSample
	for range Walk {
		live := fetch(URL(base, version, "live"))
		version = py.Str(py.Or(get(live, "patch"), ""))
		minSample = py.MustInt(py.Or(get(live, "minSample"), minSample))
		name := version
		if match := Version.FindStringSubmatch(version); match != nil {
			name = match[1]
		}
		if !seen[name] { // the newest version of its patch: its window holds the whole patch
			seen[name] = true
			cleared := pyjson.Truthy(get(py.Or(get(live, "gate"), pyjson.New()), "cleared"))
			patch := NewPatch(name, version, cleared, py.MustInt(py.Or(get(live, "totalGames"), 0)))
			add(patch.Cards, get(live, "cards"))
			if !cleared {
				dev := fetch(URL(base, version, "dev"))
				patch.Games += py.MustInt(py.Or(get(dev, "totalGames"), 0))
				add(patch.Cards, get(dev, "cards"))
			}
			if patch.Games >= MinGames {
				found = append(found, patch)
				if len(found) == Patches {
					break
				}
			}
		}
		previous := get(live, "previousPatch")
		if !pyjson.Truthy(previous) {
			break
		}
		version = py.Str(previous)
	}
	return found, minSample, version
}

// total is each card's wins and games over every patch read.
func total(patches []*Patch) []*Card {
	total := pyjson.New()
	for _, patch := range patches {
		for _, card := range cardsOf(patch.Cards) {
			into, ok := total.Get(card.ID).(*Card)
			if !ok {
				into = &Card{ID: card.ID, Name: card.Name, Set: card.Set}
				total.Set(card.ID, into)
			}
			into.Wins += card.Wins
			into.Games += card.Games
		}
	}
	var out []*Card
	for _, card := range cardsOf(total) {
		if card.Games != 0 {
			out = append(out, card)
		}
	}
	return out
}

// Rows is each set over every patch read, highest pooled rate first.
func Rows(patches []*Patch, minSample int) []*SetRow {
	cards := total(patches)
	sets := map[string]bool{}
	var names []string
	for _, card := range cards {
		if !sets[card.Set] {
			sets[card.Set] = true
			names = append(names, card.Set)
		}
	}
	sort.Strings(names)
	var out []*SetRow
	for _, name := range names {
		var mine []*Card
		for _, card := range cards {
			if card.Set == name {
				mine = append(mine, card)
			}
		}
		byPatch := pyjson.New()
		for _, patch := range patches {
			wins, games := 0, 0
			for _, card := range cardsOf(patch.Cards) {
				if card.Set == name {
					games += card.Games
				}
			}
			for _, card := range cardsOf(patch.Cards) {
				if card.Set == name {
					wins += card.Wins
				}
			}
			if games != 0 {
				byPatch.Set(patch.Name, float64(wins)/float64(games))
			} else {
				byPatch.Set(patch.Name, nil)
			}
		}
		row := &SetRow{Name: name, Cards: len(mine), ByPatch: byPatch, Rates: []float64{}}
		for _, c := range mine {
			row.Games += c.Games
		}
		for _, c := range mine {
			row.Wins += c.Wins
		}
		for _, c := range mine {
			if c.Games >= minSample {
				row.Rates = append(row.Rates, float64(c.Wins)/float64(c.Games))
			}
		}
		out = append(out, row)
	}
	pooled := func(row *SetRow) float64 {
		if p := row.Pooled(); p != nil {
			return *p
		}
		return 0
	}
	sort.SliceStable(out, func(i, j int) bool { return -pooled(out[i]) < -pooled(out[j]) })
	return out
}

// Outlier is one of `outliers`: a card and how many standard errors it is from 50%.
type Outlier struct {
	Card *Card
	Z    float64
}

// Outliers is the cards `OUTLIER_Z` or more standard errors from 50% over every patch read,
// furthest first.
func Outliers(patches []*Patch, minSample int) []Outlier {
	var out []Outlier
	for _, card := range total(patches) {
		if card.Games >= minSample {
			z := (float64(card.Wins)/float64(card.Games) - 0.5) / sqrt(0.25/float64(card.Games))
			if math.Abs(z) >= OutlierZ {
				out = append(out, Outlier{card, z})
			}
		}
	}
	sort.SliceStable(out, func(i, j int) bool { return -math.Abs(out[i].Z) < -math.Abs(out[j].Z) })
	return out
}

func pct(value *float64) string {
	if value != nil {
		return py.Percent(*value, 1)
	}
	return "—"
}

func ratePct(value any) string {
	if f, ok := value.(float64); ok {
		return pct(&f)
	}
	return "—"
}

func best(table []*SetRow, key func(*SetRow) *float64) string {
	found := false
	var top float64
	var name string
	for _, row := range table {
		k := key(row)
		if k == nil {
			continue
		}
		if !found || *k > top || (*k == top && row.Name > name) {
			found, top, name = true, *k, row.Name
		}
	}
	if !found {
		return "—"
	}
	return "**" + name + "**"
}

func when(ctx *hctx.Context) string {
	return ctx.Now().In(clock.Zone(ctx.Cfg.Timezone)).Format("2006-01-02 15:04")
}

// Draw is the section for figures just read.
func Draw(ctx *hctx.Context, patches []*Patch, minSample int, oldest string) string {
	return draw(when(ctx), patches, minSample, oldest)
}

// draw is Draw with its time written (`_when`), so the tests can draw without a clock.
func draw(when string, patches []*Patch, minSample int, oldest string) string {
	lines := []string{Start, Heading, ""}
	if len(patches) == 0 {
		back := oldest
		if back == "" {
			back = "the first"
		}
		return strings.Join(append(lines,
			fmt.Sprintf("_Read at %s from production's card statistics (`/api/stats/cards`): no "+
				"patch back to %s has %s games yet. Read again once "+
				"a day._", when, back, py.Thousands(MinGames)), "", End), "\n")
	}
	names := make([]string, len(patches))
	for i, p := range patches {
		kind := "live and AI"
		if p.Live {
			kind = "live"
		}
		names[i] = fmt.Sprintf("%s (%s %s games)", p.Name, py.Thousands(p.Games), kind)
	}
	listed := names[0]
	if len(names) > 1 {
		listed = strings.Join(names[:len(names)-1], ", ") + " and " + names[len(names)-1]
	}
	which := "the one patch"
	if len(patches) > 1 {
		which = fmt.Sprintf("the last %d patches", len(patches))
	}
	lines = append(lines, fmt.Sprintf("_Read at %s from production's card statistics (`/api/stats/cards`), once "+
		"a day: %s with %s games or more, %s._", when, which, py.Thousands(MinGames), listed), "")
	table := Rows(patches, minSample)
	patchNames := make([]string, len(patches))
	for i, patch := range patches {
		patchNames[i] = patch.Name
	}
	lines = append(lines, "| Set | Cards | Card-games | Pooled | Mean | Median | "+
		strings.Join(patchNames, " | ")+" |",
		"|---|---:|---:|---:|---:|---:|"+strings.Repeat("---:|", len(patches)))
	for _, row := range table {
		error := ""
		if e := row.Error(); e != nil {
			error = fmt.Sprintf(" ±%.1f", *e*100)
		}
		byPatch := make([]string, len(patches))
		for i, patch := range patches {
			byPatch[i] = ratePct(row.ByPatch.Get(patch.Name))
		}
		lines = append(lines, fmt.Sprintf("| %s | %d | %s | %s%s | "+
			"%s | %s | ", row.Name, row.Cards, py.Thousands(row.Games), pct(row.Pooled()), error,
			pct(row.Mean()), pct(row.Median()))+strings.Join(byPatch, " | ")+" |")
	}
	lines = append(lines, "", fmt.Sprintf("Highest pooled: %s; mean: "+
		"%s; median: "+
		"%s.", best(table, (*SetRow).Pooled), best(table, (*SetRow).Mean), best(table, (*SetRow).Median)), "")
	far := Outliers(patches, minSample)
	if len(far) > 0 {
		lines = append(lines, fmt.Sprintf("<details><summary>Cards %s or more standard errors from 50%% "+
			"(%d)</summary>", strconv.FormatFloat(OutlierZ, 'g', 6, 64), len(far)), "",
			"| Card | Set | Win rate | Games | z |", "|---|---|---:|---:|---:|")
		for _, o := range far[:min(OutliersShown, len(far))] {
			rate := float64(o.Card.Wins) / float64(o.Card.Games)
			lines = append(lines, fmt.Sprintf("| %s (`%s`) | %s | %s | "+
				"%s | %+.1f |", o.Card.Name, o.Card.ID, o.Card.Set, pct(&rate), py.Thousands(o.Card.Games), o.Z))
		}
		if len(far) > OutliersShown {
			lines = append(lines, "", fmt.Sprintf("_… and %d nearer 50%%, in `/api/stats/cards`._", len(far)-OutliersShown))
		}
		lines = append(lines, "", "</details>", "")
	}
	lines = append(lines, "<sub>A card's win rate is its deck's, over the games it was in a deck (R377); a "+
		"set's pooled rate is its cards' wins over their games, its mean and median over "+
		fmt.Sprintf("the cards with %d games or more. Decks mix sets, so this is how a set's ", minSample)+
		"cards fare in mixed decks. ± is one standard error as if every card-game were "+
		"independent, which they are not, so sets less than two or three ± apart are within "+
		"noise.</sub>", "", End)
	return strings.Join(lines, "\n")
}

// Kept is the section a count of the statistics keeps between reads, or "" (None).
func Kept(body string) string {
	start, end := strings.Index(body, Start), strings.Index(body, End)
	if start < 0 || end < start {
		return ""
	}
	return body[start : end+len(End)]
}

// Due is cardsets.due.
func Due(ctx *hctx.Context, state *pyjson.Object) bool {
	last := state.Obj("card_sets")
	at := time.Time{}
	if v := last.Get("at"); pyjson.Truthy(v) {
		at = clock.ParseISO(py.Str(v))
	}
	wait := Retry
	if last.Truthy("ok") {
		wait = Every
	}
	return at.IsZero() || ctx.Now().Sub(at) >= wait
}

func noted(ctx *hctx.Context, ok bool) {
	at := clock.ISO(ctx.Now())
	ctx.Store.Update(func(s *pyjson.Object) any {
		s.Set("card_sets", pyjson.New("at", at, "ok", ok))
		return nil
	}, "card sets")
}

// Section is the section for a count of the statistics: `old` until a read is due (or with no
// `fetch`), else read afresh. A failed read keeps `old`, or an empty section, and says it failed.
// An empty `old` is None.
func Section(ctx *hctx.Context, state *pyjson.Object, old string, fetch Fetch) string {
	if fetch == nil || (old != "" && !Due(ctx, state)) {
		return old
	}
	var text string
	if exc, failed := herr.Catch[*ReadError](func() {
		patches, minSample, oldest := Read(fetch, API)
		text = Draw(ctx, patches, minSample, oldest)
	}); failed {
		noted(ctx, false)
		note := fmt.Sprintf("> ⚠️ The read at %s failed (%s: "+
			"%s); it is tried again in "+
			"%d minutes.", when(ctx), exc.Type, py.SliceTo(strings.Join(strings.Fields(exc.Msg), " "), ErrorShown),
			int(Retry/time.Minute))
		base := old
		if base == "" {
			base = strings.Join([]string{Start, Heading, "", End}, "\n")
		}
		base = Failed.ReplaceAllString(base, "")
		return strings.Replace(base, Heading+"\n\n", Heading+"\n\n"+note+"\n\n", 1)
	}
	noted(ctx, true)
	return text
}
