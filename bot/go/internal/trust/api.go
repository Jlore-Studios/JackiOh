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
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// TRUSTED_ASSOCIATIONS = frozenset({"OWNER", "MEMBER", "COLLABORATOR"})
var TrustedAssociations any // unported

// MAX_LEVEL = 3
var MaxLevel int = 3

// LEVEL_NAMES = {0: "nobody", 1: "asker", 2: "maintainer", 3: "operator"}
var LevelNames any // unported

// Entry is trust.Entry.

type Entry struct {
	Login  string
	Level  int
	UserID *int
}

// Trust is trust.Trust.

type Trust struct {
	Entries  *pyjson.Object // default: field(default_factory=dict)
	Problems []string       // default: ()
}

func TrustParse(text string) *Trust {
	panic("unported: trust.Trust.parse")
}

func TrustLoad(path string) *Trust {
	panic("unported: trust.Trust.load")
}

// The level GitHub and the file together grant this account. 0 when either refuses.

func (x *Trust) Level(login string, userID *int /* = None */, association string /* = None */) int {
	panic("unported: trust.Trust.level")
}

func (x *Trust) Logins() []string {
	panic("unported: trust.Trust.logins")
}
