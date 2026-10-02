"""Logins: each CLI's secret written where it looks, and refreshed logins kept in the vault."""

from __future__ import annotations

import base64
import json
import stat
import tempfile
import unittest
from pathlib import Path

from harness import logins, redact, vault
from harness.errors import LoginError
from harness.providers import load as load_pool

from tests.support import ROOT

CODEX_AUTH = {"auth_mode": "chatgpt", "OPENAI_API_KEY": None, "last_refresh": "2026-10-01T00:00:00Z",
              "tokens": {"id_token": "id-" + "a" * 40, "access_token": "acc-" + "b" * 40,
                         "refresh_token": "ref-" + "c" * 40, "account_id": "acct-1234567890abcdef"}}
GEMINI_CREDS = {"access_token": "ya29." + "d" * 40, "refresh_token": "1//" + "e" * 40,
                "token_type": "Bearer", "expiry_date": 1790000000000}
MUSE_AUTH = {"schema_version": 1, "providers": {"meta": {"mechanism": "oauth",
                                                         "access_token": "m-" + "f" * 40,
                                                         "refresh_token": "mr-" + "g" * 40}}}


class VaultTests(unittest.TestCase):
    def test_round_trip_and_refusals(self):
        sealed = vault.seal({"provider": "gpt", "files": {"auth.json": "{}"}}, "secret-one")
        self.assertTrue(vault.looks_sealed(sealed))
        self.assertEqual(vault.open_(sealed, "secret-one")["provider"], "gpt")
        self.assertIsNone(vault.open_(sealed, "another-secret"))  # a new login wins
        version, tag, body = sealed.split(":", 2)
        tampered = f"{version}:{tag}:{body[:-8]}AAAAAAAA"
        self.assertIsNone(vault.open_(tampered, "secret-one"))
        self.assertFalse(vault.looks_sealed("hello"))
        self.assertFalse(vault.looks_sealed("v1:zz:" + "A" * 10))
        self.assertIsNone(vault.open_("", "secret-one"))

    def test_the_ciphertext_shows_nothing(self):
        sealed = vault.seal({"files": {"auth.json": json.dumps(CODEX_AUTH)}}, "secret-one")
        self.assertNotIn("ref-ccc", sealed)
        self.assertNotIn("chatgpt", sealed)


class PrepareTests(unittest.TestCase):
    def setUp(self):
        self.pool = load_pool(ROOT)
        self.home = Path(tempfile.mkdtemp()) / "logins"

    def test_claude_gets_its_token_and_its_own_config_directory(self):
        login = logins.prepare(self.pool.get("claude-2"), "sk-ant-oat01-" + "x" * 30, "", self.home)
        self.assertEqual(login.env["CLAUDE_CODE_OAUTH_TOKEN"], "sk-ant-oat01-" + "x" * 30)
        self.assertTrue(Path(login.env["CLAUDE_CONFIG_DIR"]).is_dir())
        self.assertEqual(login.files, {})
        self.assertIsNone(logins.seal(login))

    def test_codex_takes_auth_json_as_pasted_or_base64(self):
        for secret in (json.dumps(CODEX_AUTH),
                       base64.b64encode(json.dumps(CODEX_AUTH).encode()).decode()):
            login = logins.prepare(self.pool.get("gpt"), secret, "", self.home / secret[:4])
            auth = Path(login.env["CODEX_HOME"]) / "auth.json"
            self.assertEqual(json.loads(auth.read_text())["tokens"]["account_id"],
                             "acct-1234567890abcdef")
            self.assertEqual(stat.S_IMODE(auth.stat().st_mode), 0o600)
            self.assertIn('cli_auth_credentials_store = "file"',
                          (auth.parent / "config.toml").read_text())
        with self.assertRaises(LoginError):
            logins.prepare(self.pool.get("gpt"), "not json at all", "", self.home / "bad")

    def test_a_rotated_login_is_sealed_and_used_next_time(self):
        secret = json.dumps(CODEX_AUTH)
        login = logins.prepare(self.pool.get("gpt"), secret, "", self.home / "one")
        self.assertIsNone(logins.seal(login))  # nothing changed during the job
        rotated = json.loads(json.dumps(CODEX_AUTH))
        rotated["tokens"]["refresh_token"] = "ref-" + "z" * 40
        login.files["auth.json"].write_text(json.dumps(rotated))
        sealed = logins.seal(login)
        self.assertTrue(vault.looks_sealed(sealed))
        again = logins.prepare(self.pool.get("gpt"), secret, sealed, self.home / "two")
        self.assertEqual(again.source, "vault")
        auth = json.loads((Path(again.env["CODEX_HOME"]) / "auth.json").read_text())
        self.assertEqual(auth["tokens"]["refresh_token"], "ref-" + "z" * 40)
        # A vault under another secret is ignored: a freshly pasted login wins.
        other = logins.prepare(self.pool.get("gpt"), json.dumps({**CODEX_AUTH, "x": 1}), sealed,
                               self.home / "three")
        self.assertEqual(other.source, "secret")
        # And so is one sealed for another provider, even under the same secret.
        foreign = vault.seal({"provider": "gemini", "files": {"auth.json": "{}"}}, secret)
        mismatch = logins.prepare(self.pool.get("gpt"), secret, foreign, self.home / "four")
        self.assertEqual(mismatch.source, "secret")

    def test_gemini_gets_its_creds_and_a_settings_file_selecting_that_login(self):
        login = logins.prepare(self.pool.get("gemini"), json.dumps(GEMINI_CREDS), "", self.home)
        dot = Path(login.env["GEMINI_CLI_HOME"]) / ".gemini"
        self.assertEqual(json.loads((dot / "oauth_creds.json").read_text())["token_type"], "Bearer")
        settings = json.loads((dot / "settings.json").read_text())
        self.assertEqual(settings["security"]["auth"]["selectedType"], "oauth-personal")
        self.assertIn("AGENTS.md", settings["context"]["fileName"])

    def test_muse_takes_a_login_file_or_falls_back_to_an_api_key(self):
        login = logins.prepare(self.pool.get("muse"), json.dumps(MUSE_AUTH), "", self.home / "a")
        auth = Path(login.env["XDG_CONFIG_HOME"]) / "muse" / "auth.json"
        self.assertTrue(auth.is_file())
        self.assertNotIn("META_API_KEY", login.env)
        self.assertEqual(login.env["MUSE_NO_AUTO_UPDATE"], "1")
        keyed = logins.prepare(self.pool.get("muse"), "meta-key-" + "k" * 30, "", self.home / "b")
        self.assertEqual((keyed.env["META_API_KEY"], keyed.source), ("meta-key-" + "k" * 30, "api key"))

    def test_the_tokens_inside_a_login_file_are_redacted(self):
        logins.prepare(self.pool.get("gemini"), json.dumps(GEMINI_CREDS), "", self.home)
        text = redact.redact(f"the refresh token is {GEMINI_CREDS['refresh_token']}")
        self.assertNotIn(GEMINI_CREDS["refresh_token"], text)

    def test_an_empty_secret_is_an_error(self):
        with self.assertRaises(LoginError):
            logins.prepare(self.pool.get("claude-1"), "  ", "", self.home)


if __name__ == "__main__":
    unittest.main()
