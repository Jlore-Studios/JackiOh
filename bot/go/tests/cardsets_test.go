// Which set's cards win most, in the status issue's statistics (bot/harness/cardsets.py, #148):
// the last three patches with 1,000 games or more, each read at its newest version from
// production's `/api/stats/cards`, each set's pooled, mean and median card win rate over them,
// read once a day and kept in the issue between reads.
//
// bot/tests/test_cardsets.py, ported (#467).
package tests

import (
	"fmt"
	"net/url"
	"strings"
	"testing"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/cardsets"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/dashboard"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/fakes"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/hctx"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/stats"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/testsupport"
)

// cardsetsCard is one card of a payload: (id, name, set, wins, games).
type cardsetsCard struct {
	ID, Name, Set string
	Wins, Games   int
}

// cardsetsPayload is payload: one `/api/stats/cards` response. A nil `previous` is None.
func cardsetsPayload(version string, previous any, cleared bool, games int,
	cards []cardsetsCard) *pyjson.Object {
	rows := []any{}
	for _, c := range cards {
		var rate any // None with no games
		if c.Games != 0 {
			rate = float64(c.Wins) / float64(c.Games)
		}
		rows = append(rows, pyjson.New("id", c.ID, "name", c.Name, "set", c.Set, "games", c.Games,
			"winRate", rate))
	}
	return pyjson.New("patch", version, "previousPatch", previous,
		"gate", pyjson.New("cleared", cleared), "minSample", 20, "totalGames", games, "cards", rows)
}

// cardsetsWith is (*CORE, wins, games) and the like: a card of CORE's or CLASSIC's with figures.
func cardsetsWith(card [3]string, wins, games int) cardsetsCard {
	return cardsetsCard{card[0], card[1], card[2], wins, games}
}

// Production's history, newest first: (version, live, dev). A cleared patch's dev figures are
// its live ones, as the endpoint gives them.
var (
	cardsetsCore    = [3]string{"core-001", "Stockpile", "Core"}
	cardsetsClassic = [3]string{"classic-001", "Palantir", "Classic"}
)

// cardsetsVersion is one entry of HISTORY: a version, its live figures and its dev figures (nil:
// None).
type cardsetsVersion struct {
	Version   string
	Live, Dev *pyjson.Object
}

// cardsetsHistory is HISTORY (a fresh copy each time: the payloads are dicts).
func cardsetsHistory() []cardsetsVersion {
	core, classic := cardsetsCore, cardsetsClassic
	return []cardsetsVersion{
		{"v0.4.2b", cardsetsPayload("v0.4.2b", "v0.4.2", true, 1200,
			[]cardsetsCard{cardsetsWith(core, 700, 1200), cardsetsWith(classic, 540, 1200)}), nil},
		{"v0.4.2", cardsetsPayload("v0.4.2", "v0.4.1", true, 1000,
			[]cardsetsCard{cardsetsWith(core, 1, 1)}), nil},
		{"v0.4.1", cardsetsPayload("v0.4.1", "v0.4.0c", false, 300,
			[]cardsetsCard{cardsetsWith(core, 150, 300)}),
			cardsetsPayload("v0.4.1", "v0.4.0c", false, 400, []cardsetsCard{cardsetsWith(core, 200, 400)})},
		{"v0.4.0c", cardsetsPayload("v0.4.0c", "v0.4.0", false, 200,
			[]cardsetsCard{cardsetsWith(core, 100, 200), cardsetsWith(classic, 90, 200)}),
			cardsetsPayload("v0.4.0c", "v0.4.0", false, 900,
				[]cardsetsCard{cardsetsWith(core, 450, 900), cardsetsWith(classic, 460, 900),
					{"classic-002", "Rare", "Classic", 5, 10}})},
		{"v0.4.0", cardsetsPayload("v0.4.0", "v0.3.5", false, 1, []cardsetsCard{}), nil},
		{"v0.3.5", cardsetsPayload("v0.3.5", "v0.3.4", true, 1500,
			[]cardsetsCard{cardsetsWith(core, 750, 1500), cardsetsWith(classic, 760, 1500)}), nil},
		{"v0.3.4", cardsetsPayload("v0.3.4", nil, true, 5000, []cardsetsCard{}), nil},
	}
}

// cardsetsProduction is Production: a fake `/api/stats/cards`, recording what it was asked.
type cardsetsProduction struct {
	by     map[string]cardsetsVersion
	newest string
	asked  [][2]string
}

// newCardsetsProduction is Production(history=HISTORY); no argument is HISTORY.
func newCardsetsProduction(history ...[]cardsetsVersion) *cardsetsProduction {
	h := cardsetsHistory()
	if len(history) > 0 {
		h = history[0]
	}
	p := &cardsetsProduction{by: map[string]cardsetsVersion{}, newest: h[0].Version, asked: [][2]string{}}
	for _, v := range h {
		p.by[v.Version] = v
	}
	return p
}

// call is Production.__call__, a cardsets.Fetch.
func (p *cardsetsProduction) call(address string) any {
	parsed, err := url.Parse(address)
	if err != nil {
		panic(err)
	}
	query, err := url.ParseQuery(parsed.RawQuery)
	if err != nil {
		panic(err)
	}
	version := p.newest
	if vs, ok := query["patch"]; ok {
		version = vs[len(vs)-1] // dict(parse_qsl(...)): the last of a repeated key
	}
	sources, ok := query["source"]
	if !ok {
		panic(fmt.Sprintf("KeyError: 'source' in %s", address))
	}
	source := sources[len(sources)-1]
	p.asked = append(p.asked, [2]string{version, source})
	entry, ok := p.by[version]
	if !ok {
		panic(fmt.Sprintf("KeyError: %q", version))
	}
	if source == "dev" && entry.Dev != nil {
		return entry.Dev
	}
	return entry.Live
}

// cardsetsDraw is cardsets.draw(ctx, *cardsets.read(fetch)).
func cardsetsDraw(ctx *hctx.Context, fetch cardsets.Fetch) string {
	patches, minSample, oldest := cardsets.Read(fetch, cardsets.API)
	return cardsets.Draw(ctx, patches, minSample, oldest)
}

// ---------------------------------------------------------------------------- ReadTests

func TestCardsets_TheLastThreePatchesWith1000GamesEachReadAtItsNewestVersion(t *testing.T) {
	api := newCardsetsProduction()
	patches, minSample, oldest := cardsets.Read(api.call, cardsets.API)
	// v0.4.2 at v0.4.2b (its micro), v0.4.1 skipped at 700 games, v0.4.0 at v0.4.0c with its
	// AI games (200 live + 900), v0.3.5; the walk stops there.
	var got [][]any
	for _, p := range patches {
		got = append(got, []any{p.Name, p.Live, p.Games})
	}
	assertEqual(t, got, [][]any{{"v0.4.2", true, 1200}, {"v0.4.0", false, 1100}, {"v0.3.5", true, 1500}})
	assertEqual(t, []any{minSample, oldest}, []any{20, "v0.3.5"})
	// A cleared patch is read once (live alone); an uncleared one adds its AI games; an older
	// version of a patch already read is only walked through.
	assertEqual(t, api.asked, [][2]string{{"v0.4.2b", "live"}, {"v0.4.2", "live"}, {"v0.4.1", "live"},
		{"v0.4.1", "dev"}, {"v0.4.0c", "live"}, {"v0.4.0c", "dev"},
		{"v0.4.0", "live"}, {"v0.3.5", "live"}})
	card := patches[1].Cards.Get("core-001").(*cardsets.Card)
	assertEqual(t, []int{card.Wins, card.Games}, []int{550, 1100})
}

func TestCardsets_SetsPoolEveryCardAndTakeMeanAndMedianOverTheSampledOnes(t *testing.T) {
	patches, minSample, _ := cardsets.Read(newCardsetsProduction().call, cardsets.API)
	rows := map[string]*cardsets.SetRow{}
	var names []string
	for _, row := range cardsets.Rows(patches, minSample) {
		if _, ok := rows[row.Name]; !ok {
			names = append(names, row.Name)
		}
		rows[row.Name] = row
	}
	assertEqual(t, names, []string{"Core", "Classic"}) // highest pooled first
	core, classic := rows["Core"], rows["Classic"]
	assertEqual(t, []int{core.Cards, core.Games, core.Wins}, []int{1, 3800, 2000})
	// classic-002's 10 games count in the pool but not in the mean or median (under 20).
	assertEqual(t, []int{classic.Cards, classic.Games, classic.Wins}, []int{2, 3810, 1855})
	assertEqual(t, classic.Rates, []float64{1850.0 / 3800.0})
	assertAlmostEqual(t, core.ByPatch.Get("v0.4.2"), 700.0/1200.0)
	assertAlmostEqual(t, classic.ByPatch.Get("v0.4.0"), 555.0/1110.0)
}

func TestCardsets_TheSectionNamesTheHighestSetAndTheCardsFarFromHalf(t *testing.T) {
	ctx := testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day})
	text := cardsetsDraw(ctx, newCardsetsProduction().call)
	assertTrue(t, strings.HasPrefix(text, cardsets.Start) && strings.HasSuffix(text, cardsets.End))
	assertIn(t, "v0.4.2 (1,200 live games), v0.4.0 (1,100 live and AI games) and v0.3.5 "+
		"(1,500 live games)", text)
	assertIn(t, "| Set | Cards | Card-games | Pooled | Mean | Median | v0.4.2 | v0.4.0 | "+
		"v0.3.5 |", text)
	assertIn(t, "| Core | 1 | 3,800 | 52.6% ±0.8 | 52.6% | 52.6% | 58.3% | 50.0% | 50.0% |",
		text)
	assertIn(t, "Highest pooled: **Core**; mean: **Core**; median: **Core**.", text)
	// Stockpile, 2000 of 3800, is 3.2 standard errors over half.
	assertIn(t, "| Stockpile (`core-001`) | Core | 52.6% | 3,800 | +3.2 |", text)
	assertNotIn(t, "nearer 50%", text)
}

// The status issue keeps under GitHub's limit: `OUTLIERS_SHOWN` of the cards far from half,
// furthest first, and a line for the rest.
func TestCardsets_OnlyTheCardsFurthestFromHalfAreListed(t *testing.T) {
	extra := cardsets.OutliersShown + 3
	// Card n wins 600 + n of 1,000: z grows with n, so the last are the furthest.
	cards := []cardsetsCard{}
	for n := 0; n < extra; n++ {
		cards = append(cards, cardsetsCard{fmt.Sprintf("core-%03d", n), fmt.Sprintf("Card %d", n), "Core",
			600 + n, 1000})
	}
	history := []cardsetsVersion{
		{"v0.4.2", cardsetsPayload("v0.4.2", "v0.4.1", true, 1000, cards), nil},
		{"v0.4.1", cardsetsPayload("v0.4.1", nil, true, 10, []cardsetsCard{}), nil}}
	text := cardsetsDraw(testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day}),
		newCardsetsProduction(history).call)
	assertIn(t, fmt.Sprintf("Cards 3 or more standard errors from 50%% (%d)", extra), text)
	assertIn(t, fmt.Sprintf("Card %d (`", extra-1), text)
	assertIn(t, "Card 3 (`", text)
	assertNotIn(t, "Card 2 (`", text)
	assertIn(t, "_… and 3 nearer 50%, in `/api/stats/cards`._", text)
}

func TestCardsets_NoPatchWithEnoughGamesSaysSo(t *testing.T) {
	thin := []cardsetsVersion{{"v0.1.0", cardsetsPayload("v0.1.0", nil, false, 10, []cardsetsCard{}),
		cardsetsPayload("v0.1.0", nil, false, 10, []cardsetsCard{})}}
	text := cardsetsDraw(testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day}),
		newCardsetsProduction(thin).call)
	assertIn(t, "no patch back to v0.1.0 has 1,000 games yet", text)
	alone := []cardsetsVersion{{"v0.3.5", cardsetsPayload("v0.3.5", nil, true, 1500,
		[]cardsetsCard{cardsetsWith(cardsetsCore, 750, 1500)}), nil}}
	one := cardsetsDraw(testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day}),
		newCardsetsProduction(alone).call)
	assertIn(t, "once a day: the one patch with 1,000 games or more, v0.3.5 (1,500 live "+
		"games)._", one)
}

// ---------------------------------------------------------------------------- OnceADayTests

func TestCardsets_ReadOnceADayAndAFailedReadKeepsTheLastWithANote(t *testing.T) {
	ctx := testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day})
	gh := ctx.GH.(*fakes.FakeGitHub)
	api := newCardsetsProduction()
	first := cardsets.Section(ctx, ctx.Store.Load(), "", api.call)
	assertIn(t, "Highest pooled", first)
	assertEqual(t, ctx.Store.Load().Obj("card_sets").Get("ok"), true)
	// Within the day the section is kept, unread.
	soon := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(23 * time.Hour)})
	assertEqual(t, cardsets.Section(soon, soon.Store.Load(), first, api.call), first)
	assertEqual(t, len(api.asked), 8)
	// No fetcher (a test, Squishy's loop): kept as it is.
	assertEqual(t, cardsets.Section(soon, soon.Store.Load(), first, nil), first)

	down := func(string) any {
		panic(&cardsets.ReadError{Type: "URLError", Msg: "<urlopen error timed out>"})
	}
	later := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(24 * time.Hour)})
	failed := cardsets.Section(later, later.Store.Load(), first, down)
	assertIn(t, "> ⚠️ The read at 2026-09-30 12:00 failed (URLError: <urlopen error timed "+
		"out>); it is tried again in 60 minutes.", failed)
	assertIn(t, "Highest pooled", failed) // the last good figures stay
	// A second failure an hour on replaces the note, never stacks it.
	hour := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(25 * time.Hour)})
	again := cardsets.Section(hour, hour.Store.Load(), failed, down)
	assertEqual(t, strings.Count(again, "> ⚠️"), 1)
	assertEqual(t, strings.ReplaceAll(again, "13:00", "12:00"), failed)
	// A good read clears it.
	back := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(26 * time.Hour)})
	assertNotIn(t, "⚠️", cardsets.Section(back, back.Store.Load(), again, api.call))
	// A first read that fails still draws the section, saying so.
	bare := cardsets.Section(testsupport.MakeCtx(nil, testsupport.Ctx{At: testsupport.Day}), pyjson.New(),
		"", down)
	assertEqual(t, cardsets.Kept(bare), bare)
	assertIn(t, "failed (URLError", bare)
}

func TestCardsets_TheStatusIssueCarriesItInTheStatistics(t *testing.T) {
	gh, ctx := statsWorld()
	api := newCardsetsProduction()
	dashboard.Update(ctx, nil, true, nil, false, api.call)
	var numbers []int
	for n, thread := range gh.Threads.All() {
		if pyjson.Equal(thread.Get("title"), dashboard.Title) {
			numbers = append(numbers, n)
		}
	}
	if len(numbers) != 1 {
		t.Fatalf("status issues: %v, want one", numbers)
	}
	number := numbers[0]
	body := gh.Threads.Get(number).Str("body")
	section := cardsets.Kept(body)
	assertIn(t, section, stats.Kept(body))
	assertLess(t, strIndex(t, body, "### The last two weeks"), strIndex(t, body, cardsets.Start))
	assertLess(t, strIndex(t, body, cardsets.End), strIndex(t, body, "### Per window"))
	// The statistics are counted again in 51 minutes; the section is kept, not read again.
	asked := len(api.asked)
	later := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(51 * time.Minute)})
	dashboard.Update(later, nil, true, nil, false, api.call)
	assertEqual(t, len(api.asked), asked)
	assertEqual(t, cardsets.Kept(gh.Threads.Get(number).Str("body")), section)
	// A day on, the next count reads it afresh.
	tomorrow := testsupport.MakeCtx(gh, testsupport.Ctx{At: testsupport.Day.Add(24 * time.Hour)})
	dashboard.Update(tomorrow, nil, true, nil, false, api.call)
	assertEqual(t, len(api.asked), 2*asked)
	assertIn(t, "Read at 2026-09-30 12:00", gh.Threads.Get(number).Str("body"))
}
