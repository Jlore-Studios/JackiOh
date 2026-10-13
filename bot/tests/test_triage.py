"""Triage (bot/harness/triage.py, #664): who gets triaged, and what fixed rules give a new issue
or pull request, an issue a person labels `human`, and a thread a person calls triage on. No
model runs."""

from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path
from unittest import mock

from harness import triage
from harness.trust import Trust

from tests.fakes import FakeGitHub

BOT = "jgoetzmann-bot"
TRUST = Trust.parse("jgoetzmann 3 id:95732896\nMaxGoetzmann 3 id:87041877\nhelper 1\n")
BUILD, HUMAN, PLAN = "bot:build", "human", "bot:needs-plan"
FORM = ("### What it does\n\nA thing.\n\n### Difficulty (a guess)\n\nhard\n\n"
        "### Priority\n\nlow\n\n### Who does it\n\n{who}\n")


def event(*, action="opened", login="MaxGoetzmann", user_id=87041877, association="OWNER",
          title="Fix the thing", body="Please fix it.", labels=(), assignees=(), pr=False,
          number=40, issue_type=None, label=None, sender=None) -> dict:
    thread = {"number": number, "title": title, "body": body,
              "user": {"login": login, "id": user_id}, "author_association": association,
              "labels": [{"name": n} for n in labels],
              "assignees": [{"login": a} for a in assignees], "state": "open",
              "type": {"name": issue_type} if issue_type else None}
    payload = {"action": action, "pull_request" if pr else "issue": thread}
    if label is not None:
        payload["label"] = {"name": label}
    if sender is not None:
        payload["sender"] = {"login": sender}
    return payload


def thread(**fields) -> dict:
    return triage.thread_of(event(**fields))[0]


class GateTests(unittest.TestCase):
    def go(self, payload, asked=False):
        return triage.gate(payload, TRUST, BOT, asked=asked)

    def test_a_thread_a_trusted_person_opens_goes(self):
        self.assertEqual(self.go(event()), ("open", "#40 by @MaxGoetzmann"))
        self.assertEqual(self.go(event(pr=True))[0], "open")
        # A pinned trust line counts whatever GitHub calls the account.
        self.assertEqual(self.go(event(login="jgoetzmann", user_id=95732896,
                                       association="NONE"))[0], "open")

    def test_a_strangers_thread_and_the_bots_own_stop(self):
        step, why = self.go(event(login="someone", user_id=1, association="NONE"))
        self.assertEqual(step, "")
        self.assertIn("not trusted", why)
        # An unpinned trust line needs GitHub's word too.
        self.assertEqual(self.go(event(login="helper", user_id=2, association="CONTRIBUTOR"))[0],
                         "")
        self.assertEqual(self.go(event(login=BOT, association="MEMBER")),
                         ("", f"opened by {BOT}, which labels its own"))
        self.assertEqual(self.go(event(login="github-actions[bot]", association="NONE"))[0], "")

    def test_a_persons_human_label_hands_any_issue_to_people(self):
        """Whoever opened it: only someone with triage access can label."""
        mine = dict(login=BOT, association="MEMBER", action="labeled", label="Human")
        self.assertEqual(self.go(event(**mine, sender="MaxGoetzmann")),
                         ("human", "#40: @MaxGoetzmann labelled it human"))
        # The bot's own label, an app's, or one nobody is known to have put on, stops.
        for sender in (BOT, "github-actions[bot]", None):
            self.assertEqual(self.go(event(**mine, sender=sender))[0], "", sender)
        # Any other label, or `human` on a pull request, asks nothing of triage.
        self.assertEqual(self.go(event(action="labeled", label=BUILD, sender="MaxGoetzmann"))[0],
                         "")
        self.assertEqual(self.go(event(action="labeled", label=HUMAN, sender="MaxGoetzmann",
                                       pr=True))[0], "")

    def test_a_dispatch_goes_whoever_opened_it(self):
        self.assertEqual(self.go({"issue": thread(login=BOT)}, asked=True)[0], "open")
        self.assertEqual(self.go({"issue": {}}, asked=True)[0], "")


class DecideTests(unittest.TestCase):
    def decide(self, step="open", form=True, **fields):
        is_pr = bool(fields.get("pr"))
        return triage.decide(step, thread(**fields), is_pr, BOT, form=form)

    def test_the_title_prefix_names_the_type_label(self):
        for title, label in (("Patch v0.3.X: a card", "patch"), ("Patch v0.3.Y: a fix", "patch"),
                             ("v0.4.0 (part 1 of 3): Luau", "major version"),
                             ("Night bot: the queue", "night bot"),
                             ("Architecture: the build", "architecture"),
                             ("CI: a slow job", "architecture"), ("Make it faster", "")):
            self.assertEqual(triage.type_label(title), label, title)
        self.assertEqual(self.decide(title="Patch v0.3.Y: a fix").labels, ["patch"])
        # A type label a person (or the form's default) set stays the only one.
        self.assertEqual(self.decide(title="Night bot: x", labels=("architecture",)).labels, [])

    def test_a_form_answers_with_labels_and_human_goes_to_people(self):
        bot = self.decide(title="Patch v0.3.X: x", labels=("patch",),
                          body=FORM.format(who="The night bot (bot:build)"))
        self.assertEqual(bot.labels, ["difficulty:hard", "priority:low", BUILD])
        self.assertEqual(bot.assignees, [])
        people = self.decide(body=FORM.format(who="People (human)"), assignees=("jgoetzmann",))
        self.assertEqual(people.labels, ["difficulty:hard", "priority:low", HUMAN])
        self.assertEqual(people.assignees, ["MaxGoetzmann"])
        later = self.decide(body=FORM.format(who="Decide later"))
        self.assertEqual(later.labels, ["difficulty:hard", "priority:low"])
        # A dispatch triages it again without its form: a person may have changed those since.
        self.assertEqual(self.decide(body=FORM.format(who="People (human)"), form=False).labels,
                         [])

    def test_an_issue_gets_a_type_and_a_persons_type_stays(self):
        self.assertEqual(self.decide(title="The board crashes on a trap").issue_type, "Bug")
        self.assertEqual(self.decide(title="Night bot: a thing").issue_type, "Task")
        self.assertEqual(self.decide(title="A new leaderboard page").issue_type, "Feature")
        self.assertEqual(self.decide(title="It crashes", issue_type="Task").issue_type, "")
        self.assertEqual(self.decide(title="It crashes", pr=True).issue_type, "")

    def test_human_moves_it_to_people_and_off_the_bots_queue(self):
        plan = self.decide(step="human", labels=(HUMAN, BUILD, PLAN, "bot:working", "patch"),
                           assignees=(BOT, "MaxGoetzmann"))
        self.assertEqual(plan.assignees, ["jgoetzmann"])
        self.assertEqual(plan.unassign, [BOT])
        self.assertEqual(plan.remove, [BUILD, PLAN])  # a run's `bot:working` stays
        gone = self.decide(step="human", labels=(BUILD,))
        self.assertTrue(gone.empty())
        self.assertIn("`human` came off again", gone.notes[0])

    def test_a_pull_request_gets_only_its_type_label(self):
        plan = self.decide(pr=True, title="Night bot: x", labels=(HUMAN,),
                           body=FORM.format(who="People (human)"))
        self.assertEqual((plan.labels, plan.assignees, plan.issue_type), (["night bot"], [], ""))

    def test_a_closed_thread_changes_nothing(self):
        closed = thread(title="Patch v0.3.Y: x")
        closed["state"] = "closed"
        plan = triage.decide("open", closed, False, BOT)
        self.assertTrue(plan.empty())


class ApplyTests(unittest.TestCase):
    def test_apply_makes_each_change_and_one_refusal_leaves_the_rest(self):
        gh = FakeGitHub()
        gh.add_issue(50, labels=(BUILD,))
        gh.threads[50]["assignees"] = [{"login": BOT}]
        plan = triage.Plan(labels=["patch", HUMAN], assignees=["MaxGoetzmann"], unassign=[BOT],
                           issue_type="Bug", remove=[BUILD])
        done = triage.apply(gh, 50, plan)
        self.assertEqual(gh.label_names(50), {"patch", HUMAN})
        self.assertEqual([a["login"] for a in gh.threads[50]["assignees"]], ["MaxGoetzmann"])
        self.assertEqual(gh.threads[50]["type"], "Bug")
        self.assertIn("took `bot:build` off", done)
        with mock.patch.object(gh, "update_issue", lambda *a, **k: {"type": None}):
            done = triage.apply(gh, 50, triage.Plan(issue_type="Task", labels=["night bot"]))
        self.assertIn("could not do this: typed it Task (GitHub did not keep it)", done)
        self.assertIn("night bot", gh.label_names(50))


class CommandTests(unittest.TestCase):
    """`python -m harness triage`, as the workflow's one job runs it."""

    def run_triage(self, gh, payload):
        import argparse
        import contextlib
        import io
        import harness.__main__ as main_mod
        from tests.support import make_config, make_ctx
        cfg = make_config()
        path = Path(tempfile.mkdtemp()) / "event.json"
        path.write_text(json.dumps(payload))
        out = io.StringIO()
        with mock.patch.object(main_mod, "_ctx", lambda cfg, write=True: make_ctx(gh, cfg=cfg)), \
                contextlib.redirect_stdout(out):
            self.assertEqual(main_mod.cmd_triage(cfg, argparse.Namespace(payload=str(path))), 0)
        return out.getvalue()

    def test_a_form_issue_is_labelled_typed_and_handed_on_as_it_opens(self):
        gh = FakeGitHub()
        gh.add_issue(126, "Patch v0.3.Y: the board crashes", labels=("patch",),
                     body=FORM.format(who="The night bot (bot:build)"))
        gh.threads[126]["author_association"] = "MEMBER"
        printed = self.run_triage(gh, {"action": "opened", "issue": gh.get_issue(126)})
        self.assertEqual(gh.label_names(126), {"patch", "difficulty:hard", "priority:low", BUILD})
        self.assertEqual(gh.threads[126]["type"], "Bug")
        self.assertEqual(gh.comments[126], [])  # it says nothing on the thread
        self.assertIn("triage: open: #126 by @", printed)

    def test_a_persons_human_label_on_the_bots_issue_goes_to_people(self):
        gh = FakeGitHub()
        gh.add_issue(650, "A suggestion", labels=(HUMAN, BUILD), user={"login": BOT, "id": 1})
        gh.threads[650]["assignees"] = [{"login": BOT}]
        self.run_triage(gh, {"action": "labeled", "label": {"name": HUMAN},
                             "sender": {"login": "MaxGoetzmann"}, "issue": gh.get_issue(650)})
        self.assertEqual(gh.label_names(650), {HUMAN})
        self.assertEqual(sorted(a["login"] for a in gh.threads[650]["assignees"]),
                         ["MaxGoetzmann", "jgoetzmann"])

    def test_a_dispatch_triages_the_thread_it_names_and_a_bad_one_skips(self):
        gh = FakeGitHub()
        gh.add_issue(130, "Night bot: a thing", user={"login": BOT, "id": 1},
                     body=FORM.format(who="People (human)"))
        self.run_triage(gh, {"inputs": {"number": "#130"}})
        self.assertEqual(gh.label_names(130), {"night bot"})  # no form answers on a dispatch
        printed = self.run_triage(gh, {"inputs": {"number": "9999"}})
        self.assertIn("could not triage", printed)

    def test_a_strangers_issue_is_left_alone(self):
        gh = FakeGitHub()
        gh.add_issue(131, "Patch v0.3.Y: x", user={"login": "someone", "id": 5})
        gh.threads[131]["author_association"] = "NONE"
        printed = self.run_triage(gh, {"action": "opened", "issue": gh.get_issue(131)})
        self.assertIn("triage: skip:", printed)
        self.assertEqual(gh.label_names(131), set())


if __name__ == "__main__":
    unittest.main()
