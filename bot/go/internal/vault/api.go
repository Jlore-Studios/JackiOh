// Package vault is bot/harness/vault.py.
//
// Refreshed logins, kept encrypted between runs.
//
// Some CLIs rotate their login as they use it: Codex replaces its refresh token on every refresh,
// so the `auth.json` pasted into a secret stops working after one run unless the refreshed file is
// kept. Runners are thrown away, and the state branch is public, so the refreshed files are
// encrypted before they leave the model job, and `deliver` (which holds the bot's token but no key)
// stores the ciphertext on `bot-state` as `vault/<provider>.enc`. The next run's `plan` passes it
// back, and the model job decrypts it with a key derived from that provider's own secret. A run
// can therefore open only its own provider's vault, never another's, and a new secret (a fresh
// login pasted in) makes the old vault unreadable, so the fresh login wins.
//
// The cipher is AES-256-CBC through the `openssl` command (the standard library has none), with a
// SHA-256 HMAC over the ciphertext so a tampered or truncated vault is refused, not half-used.
package vault

import (
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// VERSION = "v1"
var Version string = "v1"

// MAX_BYTES = 200_000
var MaxBytes int = 200000

// PBKDF2_ITER = "200000"
var Pbkdf2Iter string = "200000"

// `payload` (JSON-able) encrypted under a key from `secret`, as one line of text.

func Seal(payload *pyjson.Object, secret string) string {
	panic("unported: vault.seal")
}

// The payload sealed in `text`, or None when it is missing, tampered with, or sealed under
// another secret.

func Open(text string, secret string) *pyjson.Object {
	panic("unported: vault.open_")
}

// A cheap check `deliver` makes before storing a vault it cannot open: the right shape and
// size, so a model cannot park arbitrary text on the state branch.

func LooksSealed(text string) bool {
	panic("unported: vault.looks_sealed")
}
