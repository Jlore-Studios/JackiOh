package usage

import (
	"errors"
	"io/fs"
	"maps"
	"os"
	"path/filepath"
	"regexp"
	"slices"
	"strconv"
	"strings"
	"time"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/git"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/herr"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/journal"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/providers"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// BackfillSince is the first day the backfill reads from the journal: the baseline's (#2749).
var BackfillSince = time.Date(2026, 10, 6, 0, 0, 0, 0, time.UTC)

// MarkerPath is the backfill's marker on the branch (`usage/backfill.json`): what it read and
// when. The loop backfills a branch that has none, once (`Recorder`); `harness usage backfill`
// reads the journal again whenever a person asks, and adds only the rows the branch lacks.
var MarkerPath = filepath.Join(Dir, "backfill.json")

// HasMarker is: the history under `root` was backfilled.
func HasMarker(root string) bool {
	_, err := os.Stat(filepath.Join(root, MarkerPath))
	return err == nil
}

// WriteMarker writes the backfill's marker under `root`.
func WriteMarker(root string, marker *pyjson.Object) error {
	path := filepath.Join(root, MarkerPath)
	if err := os.MkdirAll(filepath.Dir(path), 0o777); err != nil {
		return err
	}
	return os.WriteFile(path, []byte(pyjson.DumpsIndent(marker, 2)+"\n"), 0o666)
}

// FetchJournal is the journal branch's files (`<issue>.md` to its text), read from a shallow fetch
// of the branch's tip into a new directory under `parent` (removed after), not file by file through
// the Contents API; and the commit read. `token` ("" for none) is handed to git for GitHub.
func FetchJournal(parent, remote, branch, token string) (map[string]string, string, error) {
	if err := os.MkdirAll(parent, 0o777); err != nil {
		return nil, "", err
	}
	dir, err := os.MkdirTemp(parent, "journal-")
	if err != nil {
		return nil, "", err
	}
	defer os.RemoveAll(dir)
	var env map[string]string
	if token != "" {
		env = git.AuthEnv(token)
	}
	g := git.NewGit(dir, nil)
	files := map[string]string{}
	sha := ""
	err = herr.Try(func() {
		g.Run(true, GitTimeout, nil, "init", "--quiet")
		g.Run(true, GitTimeout, env, "fetch", "--quiet", "--no-tags", "--depth=1", remote,
			"+refs/heads/"+branch+":refs/remotes/journal")
		g.Run(true, GitTimeout, nil, "checkout", "--quiet", "refs/remotes/journal")
		sha = py.Strip(g.Run(true, GitTimeout, nil, "rev-parse", "HEAD").Stdout)
		entries, err := os.ReadDir(dir)
		herr.Must(err)
		for _, entry := range entries {
			if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".md") {
				continue
			}
			text, err := os.ReadFile(filepath.Join(dir, entry.Name()))
			if errors.Is(err, fs.ErrNotExist) {
				continue
			}
			herr.Must(err)
			files[entry.Name()] = string(text)
		}
	})
	return files, sha, err
}

// Backfill is every run in `files` (the journal's, name to text) that a section dated at or after
// `since` records, and a reading for each cut-off whose line names the reading and its reset (or a
// refusal and its end); and the marker that says what was read (`commit`, at `now`).
func Backfill(files map[string]string, pool *providers.Pool, since time.Time, branch, commit string,
	now time.Time) ([]Row, *pyjson.Object) {
	var rows []Row
	runs, readings := 0, 0
	for _, name := range slices.Sorted(maps.Keys(files)) {
		r, cut := ParseJournal(files[name], pool, since)
		for _, run := range r {
			rows = append(rows, run)
		}
		for _, reading := range cut {
			rows = append(rows, reading)
		}
		runs, readings = runs+len(r), readings+len(cut)
	}
	marker := pyjson.New("read", branch, "commit", commit, "since", since.Format("2006-01-02"),
		"at", ISO(now), "files", len(files), "runs", runs, "readings", readings)
	return rows, marker
}

// The lines of a journal section (`journal.Section`) the backfill reads. An older section lacks
// some (no tokens, no steps, no session): their cells stay empty.
var (
	headingLine = regexp.MustCompile("^## (\\d{4}-\\d{2}-\\d{2}) (\\d{2}):(\\d{2}) UTC · (\\S+) #(\\d+) · `([^`]+)` · (\\S+)\\s*$")
	endedLine   = regexp.MustCompile("^(?:\\[run\\]\\(([^)]*)\\)|a run) on `[^`]+` \\((.*?)\\)\\. It ended \\*\\*([^*]+)\\*\\*(?:: (.*))?$")
	modelName   = regexp.MustCompile("`([^`]+)` \\(")
	runLink     = regexp.MustCompile(`actions/runs/(\d+)`)
	callsLine   = regexp.MustCompile(`^Model calls: (\d+), (\d+(?:\.\d+)?) model minutes\.`)
	tokensLine  = regexp.MustCompile(`^Tokens: (\d+) in / (\d+) out / (\d+) cached(?:, \$(\d+(?:\.\d+)?))?\.`)
	stopStep    = regexp.MustCompile(`^\| \d+ \| (\d{2}):(\d{2}) \| stop \|`)
	cutOffWords = regexp.MustCompile(`(5-hour|7-day) usage is (\d+(?:\.\d+)?)%, (?:at or over its (\d+(?:\.\d+)?)% cap|` +
		`too close to its (\d+(?:\.\d+)?)% cap|within \d+(?:\.\d+)?% of its (\d+(?:\.\d+)?)% hard cap)` +
		`.*?; it resets at (\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z)`)
	refusedWords = regexp.MustCompile(`it refused a call; its limit resets at (\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}Z)`)
)

// ParseJournal is the runs one journal file records in sections dated at or after `since`, oldest
// first, each the previous section's subscription as its previous one, and the readings its
// cut-offs name. `pool` says which CLI each subscription runs (whether a reading before a run was
// a Claude account's ping or a `/usage`); a subscription it no longer has gets no source.
func ParseJournal(text string, pool *providers.Pool, since time.Time) ([]*Run, []*Reading) {
	_, sections := journal.Split(text)
	var runs []*Run
	var readings []*Reading
	previous := ""
	for _, section := range sections {
		run, reading := parseSection(section, pool)
		if run == nil {
			continue
		}
		run.PreviousSubscription, previous = previous, run.Subscription
		if run.EndedAt.Before(since) {
			continue
		}
		runs = append(runs, run)
		if reading != nil {
			readings = append(readings, reading)
		}
	}
	return runs, readings
}

// parseSection is one section's run, and the reading its cut-off names (nil for none); a nil run
// when the section has no heading the backfill can read.
func parseSection(section string, pool *providers.Pool) (*Run, *Reading) {
	var run *Run
	var stopAt []string
	reason := ""
	for _, text := range py.Splitlines(section) {
		if strings.HasPrefix(text, "<details>") {
			break // the builder's notes: none of the section's facts
		}
		if run == nil {
			m := headingLine.FindStringSubmatch(text)
			if m == nil {
				continue
			}
			at, err := time.Parse("2006-01-02 15:04", m[1]+" "+m[2]+":"+m[3])
			if err != nil {
				return nil, nil
			}
			run = &Run{EndedAt: at.UTC(), Kind: Word(m[4]), Item: m[5], Subscription: Word(m[6]),
				Outcome: Word(m[7]), Session: "none"}
			continue
		}
		if m := endedLine.FindStringSubmatch(text); m != nil {
			if id := runLink.FindStringSubmatch(m[1]); id != nil {
				run.RunID = id[1]
			}
			var models []string
			for _, name := range modelName.FindAllStringSubmatch(m[2], -1) {
				if model := Word(name[1]); model != "" && !slices.Contains(models, model) {
					models = append(models, model)
				}
			}
			run.Models = strings.Join(models, "+")
			run.Outcome, reason = Word(m[3]), strings.ReplaceAll(m[4], `\|`, "|")
		} else if m := callsLine.FindStringSubmatch(text); m != nil {
			run.ModelCalls, run.ModelMinutes = parseNum(m[1]), parseNum(m[2])
		} else if m := tokensLine.FindStringSubmatch(text); m != nil {
			run.TokensIn, run.TokensOut, run.TokensCached = parseNum(m[1]), parseNum(m[2]), parseNum(m[3])
			if cost := parseNum(m[4]); cost.OK && cost.V > 0 {
				run.USDReported = cost
			}
		} else if strings.HasPrefix(text, "Session: resumed") {
			run.Session = "resumed"
		} else if strings.HasPrefix(text, "Session: fresh") {
			run.Session = "fresh"
		} else if m := stopStep.FindStringSubmatch(text); m != nil {
			stopAt = m[1:]
		}
	}
	if run == nil {
		return nil, nil
	}
	run.StopReason = StopReason(run.Outcome, "", reason)
	// The cut-off's reading was taken as the run stopped: its stop step's clock, on the heading's
	// day (or the day before, for a stop just before midnight delivered after it).
	observed := run.EndedAt
	if stopAt != nil {
		hour, _ := strconv.Atoi(stopAt[0])
		minute, _ := strconv.Atoi(stopAt[1])
		at := time.Date(observed.Year(), observed.Month(), observed.Day(), hour, minute, 0, 0, time.UTC)
		if at.After(observed) {
			at = at.AddDate(0, 0, -1)
		}
		observed = at
	}
	return run, cutOffReading(run, reason, observed, pool)
}

// cutOffReading is the reading a cut-off's reason names (`providers.Refusal`'s words, after the
// run's own: "`claude-7` stops mid-call: 5-hour usage is 50%, at or over its 50% cap …; it resets
// at …"), or the refusal it names and its end; nil when it names neither.
func cutOffReading(run *Run, reason string, at time.Time, pool *providers.Pool) *Reading {
	during := "call"
	if strings.Contains(reason, "stops before it starts") {
		during = "before"
	}
	row := &Reading{ObservedAt: at, Subscription: run.Subscription, During: during, RunID: run.RunID}
	if m := refusedWords.FindStringSubmatch(reason); m != nil {
		row.Source, row.Status, row.RefusedUntil = "refusal", "rejected", ParseTime(m[1])
		return row
	}
	m := cutOffWords.FindStringSubmatch(reason)
	if m == nil {
		return nil
	}
	used, reset := parseNum(m[2]), ParseTime(m[6])
	limit := parseNum(m[3] + m[4] + m[5]) // one of the three is there
	if m[1] == "5-hour" {
		row.Util5h, row.Resets5h, row.Cap5h = used, reset, limit
	} else {
		row.Util7d, row.Resets7d, row.Cap7d = used, reset, limit
	}
	switch cli := cliOf(pool, run.Subscription); {
	case cli == "":
	case cli == "muse" || cli == "agy":
		row.Source = "usage-command"
	case cli == "claude" && during == "before":
		row.Source = "ping"
	default:
		row.Source = "stream"
	}
	if run.Util5hEnd.OK || run.Util7dEnd.OK {
		return row
	}
	run.Util5hEnd, run.Util7dEnd = row.Util5h, row.Util7d
	return row
}

// cliOf is the CLI `pool` says `id` runs, "" for none.
func cliOf(pool *providers.Pool, id string) string {
	if pool == nil {
		return ""
	}
	if p := pool.Get(id); p != nil {
		return p.CLI
	}
	return ""
}
