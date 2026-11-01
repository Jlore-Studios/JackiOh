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
	"bytes"
	"context"
	"crypto/hmac"
	"crypto/sha256"
	"encoding/base64"
	"encoding/hex"
	"errors"
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"
	"unicode"
	"unicode/utf8"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
	"github.com/Jlore-Studios/JackiOh/bot/go/internal/pyjson"
)

// Version = "v1"
var Version string = "v1"

// MaxBytes: a vault larger than this is refused rather than stored or opened.
var MaxBytes int = 200000

// Pbkdf2Iter = "200000"
var Pbkdf2Iter string = "200000"

// opensslTimeout is subprocess.run's timeout=60.
const opensslTimeout = 60 * time.Second

func keys(secret string) (string, []byte) {
	root := sha256.Sum256(append([]byte("jackioh-bot-vault\x00"), secret...))
	cipher := sha256.Sum256(append([]byte("cipher\x00"), root[:]...))
	mac := sha256.Sum256(append([]byte("mac\x00"), root[:]...))
	return hex.EncodeToString(cipher[:]), mac[:]
}

// timeoutError is subprocess.TimeoutExpired, which seal lets through and open_ catches.
type timeoutError struct{ msg string }

func (e *timeoutError) Error() string { return e.msg }

// opensslRun is _openssl: its error is the ValueError (a failed run) or OSError (no `openssl`)
// Python raises, with Python's message, or a *timeoutError.
func opensslRun(args []string, data []byte, password string) ([]byte, error) {
	// The vault's own reading of the environment, as the Python's: only PATH, for the child.
	path, ok := os.LookupEnv("PATH")
	if !ok {
		path = "/usr/bin:/bin"
	}
	argv := append([]string{"openssl", "enc", "-aes-256-cbc", "-pbkdf2", "-iter", Pbkdf2Iter,
		"-md", "sha256", "-pass", "env:VAULT_PASS"}, args...)
	bin, err := lookPath(path)
	if err != nil {
		return nil, err
	}
	ctx, cancel := context.WithTimeout(context.Background(), opensslTimeout)
	defer cancel()
	cmd := exec.CommandContext(ctx, bin, argv[1:]...)
	cmd.Args[0] = argv[0]
	cmd.Env = []string{"PATH=" + path, "VAULT_PASS=" + password}
	cmd.Stdin = bytes.NewReader(data)
	var stdout, stderr bytes.Buffer
	cmd.Stdout, cmd.Stderr = &stdout, &stderr
	err = cmd.Run()
	if ctx.Err() == context.DeadlineExceeded {
		return nil, &timeoutError{fmt.Sprintf("Command %s timed out after 60 seconds", py.Repr(argv))}
	}
	if err != nil {
		var exit *exec.ExitError
		if !errors.As(err, &exit) {
			return nil, err
		}
		return nil, fmt.Errorf("openssl failed: %s", py.SliceTo(py.DecodeReplace(stderr.Bytes()), 200))
	}
	return stdout.Bytes(), nil
}

// lookPath finds `openssl` as subprocess does with an `env`: each directory of that PATH in turn
// (an empty one is the working directory), the first file it may execute. The error is Python's
// FileNotFoundError, or PermissionError when only a file it may not execute was there.
func lookPath(path string) (string, error) {
	denied := false
	for _, dir := range strings.Split(path, ":") {
		candidate := filepath.Join(dir, "openssl")
		if !strings.Contains(candidate, "/") {
			candidate = "./" + candidate
		}
		st, err := os.Stat(candidate)
		if err != nil {
			continue
		}
		if st.IsDir() || st.Mode()&0o111 == 0 {
			denied = true
			continue
		}
		return candidate, nil
	}
	if denied {
		return "", errors.New("[Errno 13] Permission denied: 'openssl'")
	}
	return "", errors.New("[Errno 2] No such file or directory: 'openssl'")
}

// Seal is `payload` (JSON-able) encrypted under a key from `secret`, as one line of text. Its
// error is the ValueError or OSError Python raises (`openssl` failed or is missing, or the vault
// is too large); openssl running past its 60 seconds raises (panics with) Python's
// TimeoutExpired, which no caller catches.
func Seal(payload *pyjson.Object, secret string) (string, error) {
	plain := pyjson.DumpsWith(payload, pyjson.Options{SortKeys: true})
	cipherKey, macKey := keys(secret)
	body, err := opensslRun([]string{"-e", "-salt"}, []byte(plain), cipherKey)
	if err != nil {
		var timeout *timeoutError
		if errors.As(err, &timeout) {
			panic(err)
		}
		return "", err
	}
	tag := hmacHex(macKey, body)
	text := fmt.Sprintf("%s:%s:%s", Version, tag, base64.StdEncoding.EncodeToString(body))
	if n := py.Len(text); n > MaxBytes {
		return "", fmt.Errorf("the vault is %d bytes, over %d", n, MaxBytes)
	}
	return text, nil
}

func hmacHex(key, body []byte) string {
	mac := hmac.New(sha256.New, key)
	mac.Write(body)
	return hex.EncodeToString(mac.Sum(nil))
}

// Open is the payload sealed in `text`, or None (nil) when it is missing, tampered with, or
// sealed under another secret.
func Open(text string, secret string) *pyjson.Object {
	if text == "" || py.Len(text) > MaxBytes || secret == "" {
		return nil
	}
	parts := strings.SplitN(py.Strip(text), ":", 3)
	if len(parts) != 3 {
		return nil
	}
	version, tag, encoded := parts[0], parts[1], parts[2]
	if version != Version {
		return nil
	}
	body, ok := b64decode(encoded)
	if !ok {
		return nil
	}
	cipherKey, macKey := keys(secret)
	for _, r := range tag {
		if r > 0x7f {
			// hmac.compare_digest raises TypeError on a str that is not ASCII, and open_ does
			// not catch it (a Python quirk kept: a tag with such a character is an error, not
			// None).
			panic(errors.New("comparing strings with non-ASCII characters is not supported"))
		}
	}
	if !hmac.Equal([]byte(tag), []byte(hmacHex(macKey, body))) {
		return nil
	}
	plain, err := opensslRun([]string{"-d"}, body, cipherKey)
	if err != nil || !utf8.Valid(plain) {
		return nil
	}
	payload, err := pyjson.Loads(plain)
	if err != nil {
		return nil
	}
	found, _ := payload.(*pyjson.Object)
	return found
}

// LooksSealed is a cheap check `deliver` makes before storing a vault it cannot open: the right
// shape and size, so a model cannot park arbitrary text on the state branch.
func LooksSealed(text string) bool {
	if text == "" || py.Len(text) > MaxBytes {
		return false
	}
	parts := strings.SplitN(py.Strip(text), ":", 3)
	if len(parts) != 3 || parts[0] != Version || py.Len(parts[1]) != 64 {
		return false
	}
	if !hexInt(parts[1]) {
		return false
	}
	_, ok := b64decode(parts[2])
	return ok
}

// b64decode is base64.b64decode(s, validate=True): standard base64, padded as it must be,
// nothing outside the alphabet (Go's decoder would skip newlines).
func b64decode(s string) ([]byte, bool) {
	for i := 0; i < len(s); i++ {
		if s[i] >= 0x80 || s[i] == '\r' || s[i] == '\n' {
			return nil, false
		}
	}
	body, err := base64.StdEncoding.DecodeString(s)
	return body, err == nil
}

// hexInt is whether int(s, 16) reads `s`: Unicode whitespace around it, a sign, a `0x` prefix
// (with one underscore after it), hex digits (a Unicode decimal digit counts as its value) with
// single underscores between them.
func hexInt(s string) bool {
	var b strings.Builder
	for _, r := range s {
		switch {
		case r < 0x80:
			b.WriteRune(r)
		case py.IsSpace(r):
			b.WriteByte(' ')
		case unicode.IsDigit(r):
			b.WriteByte('0') // its value does not matter here, only that it is a digit
		default:
			b.WriteByte('?')
		}
	}
	t := strings.Trim(b.String(), " \t\n\v\f\r")
	if t != "" && (t[0] == '+' || t[0] == '-') {
		t = t[1:]
	}
	if len(t) >= 2 && t[0] == '0' && (t[1] == 'x' || t[1] == 'X') {
		t = strings.TrimPrefix(t[2:], "_")
	}
	if t == "" || t[0] == '_' || t[len(t)-1] == '_' || strings.Contains(t, "__") {
		return false
	}
	for i := 0; i < len(t); i++ {
		c := t[i]
		if c != '_' && !(c >= '0' && c <= '9') && !(c >= 'a' && c <= 'f') && !(c >= 'A' && c <= 'F') {
			return false
		}
	}
	return true
}
