# Launch compliance: the owner's runbook (issue #757)

What the code does for the five items of the pre-launch checklist in #757, and what is left to a
person; and, since #970, how to read the chat reports players file (section 6). None of this is legal advice: have a lawyer read `/terms`, `/privacy`, `/copyright` and the
age screen's wording before launch, and remove each page's `DRAFT — needs owner and legal review`
line only after that review.

## 1. DMCA agent and `/copyright`

- Register a designated agent with the U.S. Copyright Office's DMCA directory
  (copyright.gov/dmca-directory, $6). The contact is public. The agent needs a street address (a post
  office box only with the Office's waiver) and a telephone number, as well as the email.
- The site's contact email, for legal notices, data requests and questions, is `CONTACT_EMAIL` in
  `apps/web/src/routes/SiteFooter.tsx`. The footer, `/terms`, `/privacy`, `/copyright`,
  `/accessibility` and `apps/web/public/.well-known/security.txt` name it. Register the agent with
  the same email, or change all of them together. Changing it changes the Terms' and the Privacy
  Policy's text, so follow section 4.
- The registration lapses after three years. Put a renewal reminder in the calendar for a month
  before it does.
- Put the registered name, postal address and telephone number into `/copyright`, in place of the
  paragraph marked `data-owner-slot="dmca-agent"` in `apps/web/src/routes/copyright.tsx` (the email
  is already there).
  Change `COPYRIGHT_LAST_UPDATED` in the same file, and re-quote the page's text in
  `apps/web/static-pages.ts` (`PAGE_TEXT`; `src/static-pages.test.ts` fails until they match).
- The page is linked from the footer (`SiteFooter.tsx`) and from `/terms`.

## 2. The minimum age (R757000)

- `MIN_ACCOUNT_AGE_YEARS` in `crates/server/src/config.rs` is 13 (COPPA). Make it 16 if players in
  the EU are expected. Run `cargo test -p jackioh-server --test export_config` to rewrite
  `apps/web/src/wire/serverConfig.ts`, and commit both. `/terms` and `/privacy` read the constant,
  so their text follows; the change is a change to both pages, so change their dates and
  `TERMS_VERSION` too (section 4).
- Changing the minimum asks no existing account again. Each age check records the minimum it was
  checked against (`age_checks.min_age`).
- What is kept: that the check passed, when, and the minimum. Never the birth month or year, and the
  request is never logged.

### A user found to be under the minimum

1. Find the account in the Supabase dashboard (Authentication → Users) by its address.
2. If the player is in a live match or an unfinished Conquest series, wait for it to end: the
   deletion is refused until then (migration 0012).
3. Delete the user in the dashboard. That deletes from `auth.users`, which fires
   `on_auth_user_deleted` and removes the profile and every row that is only theirs, as "Delete my
   account" does (migration 0012). There is no parental-consent route.
4. Answer whoever reported it from a role address, without saying more about the account than that
   it was removed.

## 3. Accessibility

- `e2e/cypress/e2e/38-accessibility.cy.ts` runs axe-core over every route and both boards in the
  daily run, and fails on any serious or critical WCAG 2.2 A/AA violation not listed with its reason
  in `e2e/support/axe-exceptions.ts`. An exception that no longer fires fails it too, so the list
  only holds what is still true.
- Lighthouse is run by hand: Chrome's DevTools, Lighthouse, Accessibility only, on the deployed
  site. Write what it found into `/accessibility` with the date.
- `/accessibility` says what was checked, with what and when, and what falls short. It claims no
  conformance; keep it that way until a person has audited the site with assistive technology.

## 4. Changing the Terms or the Privacy Policy (R757001)

1. Edit the page. Set its `TERMS_LAST_UPDATED` (`apps/web/src/routes/terms.tsx`) or
   `PRIVACY_LAST_UPDATED` (`privacy.tsx`) to the day.
2. Set `TERMS_VERSION` in `crates/server/src/config.rs` to the terms' date, then the privacy
   policy's (`YYYY-MM-DD/YYYY-MM-DD`); `src/routes/privacy.test.tsx` fails until it matches. A page
   that already changed today keeps its date, and its half of the version takes a letter instead:
   `b` for the second change that day, then `c` (`2026-10-15b/2026-10-15`, R970001). Run the
   export test above and commit `serverConfig.ts`.
   The same test file's `AGREED_TEXT` holds a row per version, the SHA-256 of both pages' English
   text, and fails on any change to the words (a number the pages read from the configuration, such
   as `MIN_ACCOUNT_AGE_YEARS` or a retention period, included) until the version moves. Add the new
   version's row with the digest the failure prints; never edit an old row.
3. Re-quote the changed text in `apps/web/static-pages.ts`.
4. Every account is asked to agree again, on its next gated screen (never on `/account` and never in
   a match). The server deploys on merge and the production web up to a day later: until the web
   catches up, its sign-up and consent screen send the old version and are told to reload, and a
   pending account cannot redeem. Merge a terms change into production at once (`/fast-forward` on
   the `production merge` issue).
5. Each agreement is a row of `terms_acceptances` (account, version, source, time). Accounts made
   before migration 0034 hold a `baseline` row at their creation time.

## 5. Cancelling

JackiOh has no payments or subscriptions, so there is nothing to cancel; "Delete my account" on
`/account` removes an account. A paid feature must bring a cancel button in settings, renewal
reminders and a notice before charging.

## 6. Reviewing chat reports (B53, #970)

A player can report a message a friend sent them, with a reason (harassment, spam or other). Nothing
alerts anyone (each report writes one `chat.reported` line to the server's log): the reports wait in
`chat_reports` and are deleted with their message `CHAT_RETENTION_DAYS` (30) after it was sent, so
read them at least weekly.

1. From a checkout with the production environment (`DATABASE_URL` and the rest that `moderate`
   reads, `crates/server/README.md`), run
   `cargo run --release -p jackioh-server -- moderate chat-reports`. It changes nothing, and prints
   every report held, oldest first: when it was made and its reason, the reporter's and the
   sender's profile ids, and the message as stored (the word filter has already masked what it
   catches).
2. Judge each against the Terms' "Chat" section. Abuse is grounds for a ban:
   `moderate ban <sender-profile-id>` (each report prints the line). A ban lands at once: the
   account's next request, chat included, answers `account_banned`.
3. Nothing marks a report done: the next run lists it again until its message is purged.

## 7. What the card statistics keep (R1044000, R1044001)

Each finished match leaves a game record (`game_records`, R376), which names no account: both
decklists, opening hands, draws and plays, and the result. As it is written it is added into the
card statistics' totals (`record_totals`, `card_totals`, `decklist_totals`), counts with no single
game in them. The retention loop empties a record's detail `GAME_RECORD_RAW_RETENTION_DAYS` (7) after
it was written and keeps only its id, source, mode, patch and time, so the same game is not counted
twice; the totals are kept as long as the statistics are. The Privacy Policy's "for as long as the
statistics are kept" covers both, so this changed none of its words.
