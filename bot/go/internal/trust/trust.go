// Package trust is bot/harness/trust.py.
//
// Who may command the bot, read from `.harness/trust.txt`. Fails closed.
//
// Each line is `<login> <level> [id:<numeric GitHub user id>]`. A listed login is honoured only
// when GitHub also vouches for it: either the id matches the account that acted, or GitHub reports
// that account as OWNER, MEMBER or COLLABORATOR of the repository. Levels: 3 operator,
// 2 maintainer, 1 asker, 0 nobody.
package trust

import (
	"errors"
	"fmt"
	"io/fs"
	"math"
	"os"
	"regexp"
	"sort"
	"strconv"
	"strings"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// TrustedAssociations = frozenset({"OWNER", "MEMBER", "COLLABORATOR"})
var TrustedAssociations = map[string]bool{"OWNER": true, "MEMBER": true, "COLLABORATOR": true}

// MaxLevel = 3
var MaxLevel int = 3

// LevelNames = {0: "nobody", 1: "asker", 2: "maintainer", 3: "operator"}
var LevelNames = map[int]string{0: "nobody", 1: "asker", 2: "maintainer", 3: "operator"}

// LevelName is `LEVEL_NAMES.get(level, level)` as an f-string writes it.
func LevelName(level int) string {
	if name, ok := LevelNames[level]; ok {
		return name
	}
	return strconv.Itoa(level)
}

var (
	loginPattern = regexp.MustCompile(`^[A-Za-z0-9-]{1,39}$`)
	// re.IGNORECASE: Python's `i` also matches `ı` and `İ`.
	idPattern = regexp.MustCompile(`^[iIıİ][dD]:([1-9][0-9]{0,19})$`)
)

// Entry is trust.Entry.
type Entry struct {
	Login  string
	Level  int
	UserID *int
	// idTooBig: the line pins an id beyond a Go int (Python's int holds it), which no account's
	// id can equal.
	idTooBig bool
}

// Trust is trust.Trust. Entries is keyed by the lowercased login.
type Trust struct {
	Entries  map[string]*Entry // default: field(default_factory=dict)
	Problems []string          // default: ()
}

// TrustParse is Trust.parse.
func TrustParse(text string) *Trust {
	entries := map[string]*Entry{}
	problems := []string{}
	for i, raw := range py.Splitlines(text) {
		number := i + 1
		line, _, _ := strings.Cut(raw, "#")
		line = py.Strip(line)
		if line == "" {
			continue
		}
		parts := strings.FieldsFunc(line, py.IsSpace)
		login := strings.TrimLeft(parts[0], "@")
		if !loginPattern.MatchString(login) {
			problems = append(problems, fmt.Sprintf("line %d: %s is not a GitHub login", number, py.Repr(login)))
			continue
		}
		level := 1
		var userID *int
		idTooBig := false
		bad := false
		for _, token := range parts[1:] {
			if isASCIIDigits(token) {
				shown := strings.TrimLeft(token, "0")
				if shown == "" {
					shown = "0"
				}
				n, err := strconv.Atoi(token)
				if err != nil {
					n = math.MaxInt // Python's int holds it; it is above MaxLevel either way
				}
				level = n
				if level > MaxLevel {
					problems = append(problems, fmt.Sprintf("line %d: level %s is above %d", number, shown, MaxLevel))
					bad = true
				}
				continue
			}
			if match := idPattern.FindStringSubmatch(token); match != nil {
				if id, err := strconv.Atoi(match[1]); err == nil {
					userID, idTooBig = &id, false
				} else {
					userID, idTooBig = nil, true
				}
				continue
			}
			problems = append(problems, fmt.Sprintf("line %d: cannot read %s", number, py.Repr(token)))
			bad = true
		}
		if !bad {
			entries[strings.ToLower(login)] = &Entry{login, level, userID, idTooBig}
		}
	}
	return &Trust{entries, problems}
}

// TrustLoad is Trust.load. A missing file is a Trust that refuses everyone; any other error
// reading it raises (panics with) the error, as Python's OSError and UnicodeDecodeError go
// uncaught.
func TrustLoad(path string) *Trust {
	data, err := os.ReadFile(path)
	if err != nil {
		if errors.Is(err, fs.ErrNotExist) {
			return &Trust{map[string]*Entry{}, []string{fmt.Sprintf("%s does not exist; nobody may command the bot", path)}}
		}
		panic(err)
	}
	if !utf8.Valid(data) {
		panic(fmt.Errorf("'utf-8' codec can't decode %s", path))
	}
	return TrustParse(string(data))
}

// Level is the level GitHub and the file together grant this account. 0 when either refuses.
// `userID` nil is None; `association` "" is None.
func (x *Trust) Level(login string, userID *int /* = None */, association string /* = None */) int {
	entry, ok := x.Entries[py.Lower(strings.TrimLeft(login, "@"))]
	if !ok {
		return 0
	}
	if entry.UserID != nil || entry.idTooBig {
		// A pinned line counts for that account only; no id to compare is a refusal.
		if userID != nil && entry.UserID != nil && *userID == *entry.UserID {
			return entry.Level
		}
		return 0
	}
	if TrustedAssociations[strings.ToUpper(association)] {
		return entry.Level
	}
	return 0
}

func (x *Trust) Logins() []string {
	logins := []string{}
	for _, e := range x.Entries {
		logins = append(logins, e.Login)
	}
	sort.Strings(logins)
	return logins
}

func isASCIIDigits(s string) bool {
	if s == "" {
		return false
	}
	for i := 0; i < len(s); i++ {
		if s[i] < '0' || s[i] > '9' {
			return false
		}
	}
	return true
}
