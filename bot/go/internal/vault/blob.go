package vault

// Blobs: bytes kept the way a refreshed login is (#2748). A Claude run's own Claude Code session
// (`internal/sessions`) holds every tool output of the run, so it leaves the model job only
// sealed, under the same subscription's key, in the job's `work` artifact, and `deliver`, which
// holds no key, checks only its shape before it records where it is.
//
// A blob is `BlobVersion:<hmac>:<base64>`: the cipher and the keys are a login's, but its HMAC
// covers the version too, so a login's vault never opens as a blob nor a blob as a login's vault.

import (
	"crypto/hmac"
	"encoding/base64"
	"errors"
	"fmt"
	"strings"

	"github.com/Jlore-Studios/JackiOh/bot/go/internal/py"
)

// BlobVersion is the first field of a sealed blob.
var BlobVersion = "b1"

// TooLarge is a blob that, sealed, is over the limit its caller set.
type TooLarge struct {
	Size  int
	Limit int
}

func (e *TooLarge) Error() string {
	return fmt.Sprintf("sealed, it is %s bytes, over the %s-byte cap", py.Thousands(e.Size), py.Thousands(e.Limit))
}

// blobTag is the HMAC of a blob: over its version and its ciphertext.
func blobTag(macKey, body []byte) string {
	return hmacHex(macKey, append([]byte(BlobVersion+":"), body...))
}

// SealBlob is `data` encrypted under a key from `secret`, as one line of text no longer than
// `limit` characters. Its error is a *TooLarge, or `openssl`'s failure as Seal's is.
func SealBlob(data []byte, secret string, limit int) (string, error) {
	if secret == "" {
		return "", errors.New("no secret to seal it with")
	}
	cipherKey, macKey := keys(secret)
	body, err := opensslRun([]string{"-e", "-salt"}, data, cipherKey)
	if err != nil {
		return "", err
	}
	text := BlobVersion + ":" + blobTag(macKey, body) + ":" + base64.StdEncoding.EncodeToString(body)
	if len(text) > limit {
		return "", &TooLarge{Size: len(text), Limit: limit}
	}
	return text, nil
}

// OpenBlob is the bytes sealed in `text`; its error says why it would not open: missing, too
// large, tampered with, or sealed under another secret.
func OpenBlob(text string, secret string, limit int) ([]byte, error) {
	text = strings.TrimSpace(text)
	switch {
	case text == "":
		return nil, errors.New("there is nothing to open")
	case secret == "":
		return nil, errors.New("there is no secret to open it with")
	case len(text) > limit:
		return nil, &TooLarge{Size: len(text), Limit: limit}
	}
	parts := strings.SplitN(text, ":", 3)
	if len(parts) != 3 || parts[0] != BlobVersion || !isHex64(parts[1]) {
		return nil, errors.New("it is not a sealed blob")
	}
	body, ok := b64decode(parts[2])
	if !ok {
		return nil, errors.New("it is not a sealed blob")
	}
	cipherKey, macKey := keys(secret)
	if !hmac.Equal([]byte(parts[1]), []byte(blobTag(macKey, body))) {
		return nil, errors.New("it was sealed under another secret, or changed since")
	}
	plain, err := opensslRun([]string{"-d"}, body, cipherKey)
	if err != nil {
		return nil, err
	}
	return plain, nil
}

// LooksSealedBlob is the check `deliver` makes before it points the next run at a blob it cannot
// open: the right shape and size, so a model cannot hand that run arbitrary text.
func LooksSealedBlob(text string, limit int) bool {
	text = strings.TrimSpace(text)
	if text == "" || len(text) > limit {
		return false
	}
	parts := strings.SplitN(text, ":", 3)
	if len(parts) != 3 || parts[0] != BlobVersion || !isHex64(parts[1]) {
		return false
	}
	_, ok := b64decode(parts[2])
	return ok
}

// isHex64 is a SHA-256 HMAC as hex: 64 lower-case hex digits.
func isHex64(s string) bool {
	if len(s) != 64 {
		return false
	}
	for i := 0; i < len(s); i++ {
		if c := s[i]; !(c >= '0' && c <= '9') && !(c >= 'a' && c <= 'f') {
			return false
		}
	}
	return true
}
