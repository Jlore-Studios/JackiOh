//! Chat between friends (SPEC S7, B53).
//!
//! - `GET /api/chat/:friendId?before=<iso>`: the newest `CHAT_PAGE` messages of the conversation
//!   sent before `before` (every one without it), newest last; the ones sent to the caller are read
//!   from then on, and the page says so.
//! - `POST /api/chat/:friendId` `{body}`: `201 {message}`. Only between friends neither side
//!   blocked (else 404, the one answer for both). The body is trimmed; empty is 422 `empty`, more
//!   than `CHAT_MAX_CHARS` characters 422 `too_long`; then each word the username filter would
//!   refuse (R1433's two lists, folded as a name is folded) is masked with `*`, a star per
//!   character, and the message is stored as masked. Past `CHAT_MESSAGES_PER_MINUTE` a minute, 429;
//!   only a message that would be stored counts.
//! - `GET /api/chat/unread`: `{unread: {[friendId]: n}}`, each friend with unread messages to the
//!   caller.
//! - `POST /api/chat/messages/:id/report` `{reason}`: its recipient reports a message, once (a
//!   second report by the same player changes nothing). 204; 404 for anyone else.
//! - The purge (`purge`) deletes every message older than `CHAT_RETENTION_DAYS`, its reports with
//!   it, each time the retention loop runs (`api::retention::run_purge`: at boot, then hourly).
//!
//! No log line carries a message's body or a reason.

use std::sync::Arc;

use caseless::default_case_fold_str;
use serde::Serialize;
use serde_json::{Value, json};
use unicode_normalization::UnicodeNormalization;
use unicode_properties::{GeneralCategoryGroup, UnicodeGeneralCategory};

use crate::api::collection::caller_profile;
use crate::api::http::{
    ApiResult, Req, account_key, bad_request, log_info, now_ms, ok, rate_limited, str, to_json,
};
use crate::api::queue::new_uuid;
use crate::app::App;
use crate::config::{CHAT_MAX_CHARS, CHAT_PAGE, CHAT_RETENTION_DAYS};
use crate::db::store::{ChatMessageRow, ChatReportRow, StoreError};
use crate::social::friends::are_friends;
use crate::social::{MS_PER_DAY, created, iso, no_content, not_found, param, parse_iso, refused};

/// S7's `ChatMessage`: `from` and `to` are profile ids; `read` is whether its recipient has read it.
#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub from: String,
    pub to: String,
    pub body: String,
    pub created_at: String,
    pub read: bool,
}

impl ChatMessage {
    pub fn of(row: &ChatMessageRow) -> ChatMessage {
        ChatMessage {
            id: row.id.clone(),
            from: row.from_profile.clone(),
            to: row.to_profile.clone(),
            body: row.body.clone(),
            created_at: iso(row.created_at),
            read: row.read_at.is_some(),
        }
    }
}

/// The answer for a conversation the caller may not reach: not friends, or blocked either way.
fn not_a_friend() -> crate::api::http::ApiError {
    not_found("that player is not your friend")
}

/// `GET /api/chat/:friendId?before=<iso>` (B53, B58): `200 {messages}`, newest last.
pub async fn get_messages(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let friend_id = param(&req, "friendId");
    let before = match req.query.get("before").map(|text| text.trim()) {
        None | Some("") => None,
        Some(text) => {
            Some(parse_iso(text).ok_or_else(|| bad_request("\"before\" must be an ISO-8601 time"))?)
        }
    };
    let now = now_ms();

    let mut tx = app.db.begin(Some(&me)).await?;
    if !are_friends(&mut tx, &me, &friend_id).await? {
        return Err(not_a_friend());
    }
    let mut page = tx.chat_page(&me, &friend_id, before, CHAT_PAGE).await?;
    let unread: Vec<String> = page
        .iter()
        .filter(|row| row.to_profile == me && row.read_at.is_none())
        .map(|row| row.id.clone())
        .collect();
    if !unread.is_empty() {
        tx.chat_mark_read(&me, &unread, now).await?;
        for row in page.iter_mut().filter(|row| unread.contains(&row.id)) {
            row.read_at = Some(now);
        }
    }
    tx.commit().await?;
    let messages: Vec<ChatMessage> = page.iter().map(ChatMessage::of).collect();
    Ok(ok(json!({ "messages": to_json(&messages)? })))
}

/// `POST /api/chat/:friendId` `{body}` (B53): `201 {message}`.
pub async fn send(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let friend_id = param(&req, "friendId");
    let typed = match req.body.get("body") {
        None | Some(Value::Null) => String::new(),
        Some(Value::String(text)) => text.clone(),
        Some(_) => return Err(bad_request("\"body\" must be a string")),
    };

    let mut tx = app.db.begin(Some(&me)).await?;
    if !are_friends(&mut tx, &me, &friend_id).await? {
        return Err(not_a_friend());
    }
    let body = typed.trim();
    if body.is_empty() {
        return refused(422, "empty", "Write a message first.");
    }
    if body.chars().count() > CHAT_MAX_CHARS {
        return refused(
            422,
            "too_long",
            &format!("A message holds at most {CHAT_MAX_CHARS} characters."),
        );
    }
    let now = now_ms();
    let key = account_key(&me);
    if !app.social.chat.allow(&key, now) {
        return Err(rate_limited(
            "too many messages; slow down",
            app.social.chat.retry_after_ms(&key, now),
        ));
    }
    let row = ChatMessageRow {
        id: new_uuid(),
        from_profile: me.clone(),
        to_profile: friend_id.clone(),
        body: mask_blocked_words(body),
        created_at: now,
        read_at: None,
    };
    tx.chat_insert(&row).await?;
    tx.commit().await?;
    Ok(created(json!({ "message": to_json(&ChatMessage::of(&row))? })))
}

/// `GET /api/chat/unread` (B53): `200 {unread: {[friendId]: n}}`, current friends only.
pub async fn get_unread(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let mut tx = app.db.begin(Some(&me)).await?;
    let counts = tx.chat_unread(&me).await?;
    let friends: Vec<String> = tx
        .friends_of(&me)
        .await?
        .iter()
        .map(|row| row.other(&me).to_string())
        .collect();
    tx.commit().await?;
    let unread: serde_json::Map<String, Value> = counts
        .into_iter()
        .filter(|(sender, count)| *count > 0 && friends.contains(sender))
        .map(|(sender, count)| (sender, json!(count)))
        .collect();
    Ok(ok(json!({ "unread": Value::Object(unread) })))
}

/// `POST /api/chat/messages/:id/report` `{reason}` (B53): 204. `reason` is a non-empty string of at
/// most `CHAT_MAX_CHARS` characters (400 otherwise); only the message's recipient may report it.
pub async fn report(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let message_id = param(&req, "id");
    let reason = str(&req.body, "reason")?;
    let reason = reason.trim();
    if reason.is_empty() || reason.chars().count() > CHAT_MAX_CHARS {
        return Err(bad_request(format!(
            "\"reason\" must be 1 to {CHAT_MAX_CHARS} characters"
        )));
    }
    let mut tx = app.db.begin(Some(&me)).await?;
    let message = match tx.chat_get(&message_id).await? {
        Some(message) if message.to_profile == me => message,
        _ => return Err(not_found("no such message")),
    };
    let stored = tx
        .chat_report(&ChatReportRow {
            id: new_uuid(),
            message_id: message.id.clone(),
            reporter: me.clone(),
            reason: reason.to_string(),
            created_at: now_ms(),
        })
        .await?;
    tx.commit().await?;
    if stored {
        log_info(
            "chat.reported",
            json!({ "messageId": message.id, "reporterProfileId": me, "senderProfileId": message.from_profile }),
        );
    }
    Ok(no_content())
}

/// B53: deletes every message older than `CHAT_RETENTION_DAYS`; answers how many went. The retention
/// loop runs it (`api::retention::run_purge`).
pub async fn purge(app: &App) -> Result<i64, StoreError> {
    let before = now_ms() - CHAT_RETENTION_DAYS * MS_PER_DAY;
    let mut tx = app.db.begin(None).await?;
    let purged = tx.chat_purge(before).await?;
    tx.commit().await?;
    Ok(purged)
}

// ---------------------------------------------------------------------------------------------
// The word filter (B53): R1433's lists, the ones a username is held to, read over a message's words
// ---------------------------------------------------------------------------------------------

/// Words refused wherever they appear (`username/blocklist_anywhere.txt`).
const BLOCKLIST_ANYWHERE: &str = include_str!("../username/blocklist_anywhere.txt");
/// Words refused only as a whole word or a part of one (`username/blocklist_token.txt`).
const BLOCKLIST_TOKEN: &str = include_str!("../username/blocklist_token.txt");

/// One list's words: a lowercase word per line, `#` comments and blank lines skipped.
fn words(list: &'static str) -> impl Iterator<Item = &'static str> {
    list.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

fn is_mark(ch: char) -> bool {
    ch.general_category_group() == GeneralCategoryGroup::Mark
}

/// R1433's common digit substitutions: the letter a digit stands for, if it stands for one.
fn substitute(ch: char) -> Option<char> {
    match ch {
        '0' => Some('o'),
        '1' => Some('i'),
        '3' => Some('e'),
        '4' => Some('a'),
        '5' => Some('s'),
        '7' => Some('t'),
        _ => None,
    }
}

/// R1433's folding before a match, as a username is folded: the full case fold, NFD with every
/// combining mark dropped, underscores dropped, the substitution digits read as their letters.
fn fold(text: &str) -> String {
    default_case_fold_str(text)
        .nfd()
        .filter(|ch| !is_mark(*ch) && *ch != '_')
        .map(|ch| substitute(ch).unwrap_or(ch))
        .collect()
}

/// A word's parts as a username's tokens split: on `_` and where a lowercase letter meets an
/// uppercase one (`BigAss` is `Big`, `Ass`), its substitution digits read as letters first.
fn parts(word: &str) -> Vec<String> {
    let read: String = word.chars().map(|ch| substitute(ch).unwrap_or(ch)).collect();
    let mut out: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut last_lower = false;
    for ch in read.chars() {
        if ch == '_' {
            out.push(std::mem::take(&mut current));
            last_lower = false;
            continue;
        }
        if last_lower && ch.is_uppercase() {
            out.push(std::mem::take(&mut current));
        }
        current.push(ch);
        last_lower = ch.is_lowercase();
    }
    out.push(current);
    out.retain(|part| !part.is_empty());
    out
}

/// Whether the filter refuses this word: an anywhere-word inside it, or a token-word that is the
/// whole of it or of one of its parts. A word with no letter is never matched, so a number's digits
/// are never read as a word (`455` is not `ass`).
fn is_blocked_word(word: &str) -> bool {
    if !word.chars().any(char::is_alphabetic) {
        return false;
    }
    let folded = fold(word);
    if words(BLOCKLIST_ANYWHERE).any(|blocked| folded.contains(blocked)) {
        return true;
    }
    std::iter::once(folded)
        .chain(parts(word).iter().map(|part| fold(part)))
        .any(|token| words(BLOCKLIST_TOKEN).any(|blocked| token == blocked))
}

/// B53: the message with every word the filter refuses masked, a `*` per character, so its length
/// is unchanged. A word is a run of letters, digits, marks and `_`; everything between words is
/// kept as written.
pub fn mask_blocked_words(text: &str) -> String {
    fn flush(word: &mut String, out: &mut String) {
        if is_blocked_word(word) {
            out.extend(word.chars().map(|_| '*'));
        } else {
            out.push_str(word);
        }
        word.clear();
    }
    let mut out = String::with_capacity(text.len());
    let mut word = String::new();
    for ch in text.chars() {
        if ch.is_alphanumeric() || is_mark(ch) || ch == '_' {
            word.push(ch);
        } else {
            flush(&mut word, &mut out);
            out.push(ch);
        }
    }
    flush(&mut word, &mut out);
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A word the token list holds, read off the list itself so the test names no word.
    fn a_token_word() -> &'static str {
        words(BLOCKLIST_TOKEN)
            .next()
            .expect("the token list holds a word")
    }

    fn an_anywhere_word() -> &'static str {
        words(BLOCKLIST_ANYWHERE)
            .next()
            .expect("the anywhere list holds a word")
    }

    fn stars(word: &str) -> String {
        word.chars().map(|_| '*').collect()
    }

    #[test]
    fn b53_masks_a_listed_word_a_star_per_character_and_keeps_the_rest() {
        let word = a_token_word();
        let text = format!("well, {word}! good game");
        assert_eq!(
            mask_blocked_words(&text),
            format!("well, {}! good game", stars(word))
        );
        let upper = word.to_uppercase();
        assert_eq!(mask_blocked_words(&upper), stars(&upper));
    }

    #[test]
    fn b53_masks_an_anywhere_word_inside_a_longer_word() {
        let word = an_anywhere_word();
        let longer = format!("x{word}y");
        assert_eq!(mask_blocked_words(&longer), stars(&longer));
    }

    #[test]
    fn b53_leaves_clean_text_and_bare_numbers_alone() {
        assert_eq!(
            mask_blocked_words("good game, well played"),
            "good game, well played"
        );
        assert_eq!(mask_blocked_words("gg 455 0 1 3"), "gg 455 0 1 3");
        assert_eq!(mask_blocked_words(""), "");
    }

    #[test]
    fn b53_splits_a_word_into_its_parts_as_a_username_is_split() {
        assert_eq!(parts("BigWord"), vec!["Big", "Word"]);
        assert_eq!(parts("my_word"), vec!["my", "word"]);
        assert_eq!(parts("a55"), vec!["ass"]);
    }
}
