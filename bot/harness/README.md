# The Python harness, kept for the cut-over (#467)

No workflow runs this any more: the harness is `bot/go/` (`harness <command>`), and this directory is
`main`'s last Python harness, byte for byte. It stays for the hours after the move to Go because a run
GitHub started before it keeps the workflow file it started with (`python -m harness …`) while its later
jobs (work, deliver) check out `main` afresh. Without this directory those jobs would find no harness and
the run's work would be lost. The follow-up pull request that deletes it lands once every run started
before the move has finished.

Do not change it: a fix belongs in `bot/go/`.
