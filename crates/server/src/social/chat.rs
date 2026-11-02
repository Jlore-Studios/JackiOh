//! Chat between friends (SPEC S7, B53).
//!
//! - `GET /api/chat/:friendId?before=<cursor>`: `200 {messages, nextCursor}`, the newest `CHAT_PAGE`
//!   messages of the conversation that come before `before` in `(sent at, id)` order (every one
//!   without it), newest last; the ones sent to the caller are read from then on, and the page says
//!   so. `nextCursor` is the cursor of the page's oldest message when the page holds `CHAT_PAGE`,
//!   else `null` (R1384000); a `before` this server did not write is 400 before anything is read.
//! - `POST /api/chat/:friendId` `{body}`: `201 {message}`. Only between friends neither side
//!   blocked (else 404, the one answer for both). The body is trimmed; empty is 422 `empty`, more
//!   than `CHAT_MAX_CHARS` characters 422 `too_long`; then each word the username filter would
//!   refuse (R1433's `is_blocked` itself, so look-alikes and digits read as in a name) is masked
//!   with `*`, a star per character, and the message is stored as masked. Past
//!   `CHAT_MESSAGES_PER_MINUTE` a minute, 429; only a message that would be stored counts.
//! - `GET /api/chat/unread`: `{unread: {[friendId]: n}}`, each friend with unread messages to the
//!   caller.
//! - `POST /api/chat/messages/:id/report` `{reason}`: its recipient reports a message, once (a
//!   second report by the same player changes nothing). 204; 404 for anyone else.
//! - The purge (`purge`) deletes every message older than `CHAT_RETENTION_DAYS`, its reports with
//!   it, each time the retention loop runs (`api::retention::run_purge`: at boot, then hourly).
//!
//! No log line carries a message's body or a reason.

use std::sync::Arc;

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
use crate::config::{CHAT_MAX_CHARS, CHAT_PAGE, CHAT_RETENTION_DAYS, MS_PER_DAY};
use crate::db::store::{ChatCursor, ChatMessageRow, ChatReportRow, StoreError};
use crate::social::friends::are_friends;
use crate::social::{created, iso, no_content, not_found, param, refused};
use crate::username::is_blocked;

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

/// R1384000: what joins a cursor's time to its id.
const CURSOR_SEPARATOR: char = '_';

/// R1384000: the cursor a page answers, `<created_at ms>_<id>`. Opaque to the client, which only
/// sends it back.
fn cursor_text(cursor: &ChatCursor) -> String {
    format!("{}{CURSOR_SEPARATOR}{}", cursor.created_at, cursor.id)
}

/// R1384000: a cursor exactly as `cursor_text` writes one (a time of at least 0, a lowercase
/// hyphenated uuid), or `None`.
fn parse_cursor(text: &str) -> Option<ChatCursor> {
    let (time, id) = text.split_once(CURSOR_SEPARATOR)?;
    let cursor = ChatCursor {
        created_at: time.parse::<i64>().ok().filter(|at| *at >= 0)?,
        id: uuid::Uuid::parse_str(id).ok()?.to_string(),
    };
    (cursor_text(&cursor) == text).then_some(cursor)
}

/// `GET /api/chat/:friendId?before=<cursor>` (B53, B58, R1384000): `200 {messages, nextCursor}`,
/// newest last. `nextCursor` is the cursor of the page's oldest message when the page holds
/// `CHAT_PAGE`, else `null`; a `before` this server did not write is 400, before anything is read.
pub async fn get_messages(app: &Arc<App>, req: Req) -> ApiResult {
    let me = caller_profile(&req)?.id;
    let friend_id = param(&req, "friendId");
    let before = match req.query.get("before").map(|text| text.trim()) {
        None | Some("") => None,
        Some(text) => Some(
            parse_cursor(text)
                .ok_or_else(|| bad_request("\"before\" must be the nextCursor a page answered"))?,
        ),
    };
    let now = now_ms();

    let mut tx = app.db.begin(Some(&me)).await?;
    if !are_friends(&mut tx, &me, &friend_id).await? {
        return Err(not_a_friend());
    }
    let mut page = tx.chat_page(&me, &friend_id, before.as_ref(), CHAT_PAGE).await?;
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
    let full = i64::try_from(page.len()).is_ok_and(|held| held >= CHAT_PAGE);
    let next_cursor = page
        .first()
        .filter(|_| full)
        .map(|row| cursor_text(&ChatCursor::of(row)));
    let messages: Vec<ChatMessage> = page.iter().map(ChatMessage::of).collect();
    Ok(ok(
        json!({ "messages": to_json(&messages)?, "nextCursor": next_cursor }),
    ))
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
    // A conversation reads in the order it was written: a message sent in the same millisecond as
    // the newest one before it is stamped a millisecond after it, since the id that breaks a tie
    // between equal times is random.
    let newest = tx.chat_page(&me, &friend_id, None, 1).await?;
    let created_at = newest.first().map_or(now, |row| now.max(row.created_at + 1));
    let row = ChatMessageRow {
        id: new_uuid(),
        from_profile: me.clone(),
        to_profile: friend_id.clone(),
        body: mask_blocked_words(body),
        created_at,
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
// The word filter (B53): the usernames' filter (R1433), read over a message's words
// ---------------------------------------------------------------------------------------------

fn is_mark(ch: char) -> bool {
    ch.general_category_group() == GeneralCategoryGroup::Mark
}

/// Whether the filter refuses this word: the usernames' filter (R1433) over its NFKC form, look-alike
/// letters and digits read back both ways as in a name. A word with no letter is never matched, so a
/// number's digits are never read as a word (`455` is not `ass`).
fn is_blocked_word(word: &str) -> bool {
    word.chars().any(char::is_alphabetic) && is_blocked(&word.nfkc().collect::<String>())
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
    use crate::username::normalize_username;

    /// The first word of one of the usernames' lists, read off the list itself so the test names
    /// no word.
    fn first_word(list: &'static str) -> &'static str {
        list.lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .expect("the list holds a word")
    }

    /// A word the token list holds.
    fn a_token_word() -> &'static str {
        first_word(include_str!("../username/blocklist_token.txt"))
    }

    /// A word the anywhere list holds.
    fn an_anywhere_word() -> &'static str {
        first_word(include_str!("../username/blocklist_anywhere.txt"))
    }

    /// `word` in the Latin small capitals a fancy-text tool writes (`ɴɪɢɢᴇʀ`), which NFKC and the
    /// case fold leave alone.
    fn small_capitals(word: &str) -> String {
        const PLAIN: &str = "abcdefghijklmnopqrstuvwxyz";
        const SMALL: &str = "ᴀʙᴄᴅᴇꜰɢʜɪᴊᴋʟᴍɴᴏᴘꞯʀꜱᴛᴜᴠᴡxʏᴢ";
        word.chars()
            .map(|ch| PLAIN.find(ch).and_then(|at| SMALL.chars().nth(at)).unwrap_or(ch))
            .collect()
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
    fn r1384000_a_cursor_reads_back_only_as_it_is_written() {
        let id = "0f1e2d3c-4b5a-4968-8776-655443322110";
        let cursor = ChatCursor {
            created_at: 1_760_445_296_789,
            id: id.into(),
        };
        let text = "1760445296789_0f1e2d3c-4b5a-4968-8776-655443322110";
        assert_eq!(cursor_text(&cursor), text);
        assert_eq!(parse_cursor(text), Some(cursor));
        for refused in [
            "",
            "garbage",
            "2026-10-14T09:00:00.000Z",
            "1760445296789",
            "1760445296789_not-a-uuid",
            "_0f1e2d3c-4b5a-4968-8776-655443322110",
            "-1_0f1e2d3c-4b5a-4968-8776-655443322110",
            "+1_0f1e2d3c-4b5a-4968-8776-655443322110",
            "01_0f1e2d3c-4b5a-4968-8776-655443322110",
            "1_0F1E2D3C-4B5A-4968-8776-655443322110",
            "1_{0f1e2d3c-4b5a-4968-8776-655443322110}",
        ] {
            assert_eq!(parse_cursor(refused), None, "{refused:?}");
        }
    }

    #[test]
    fn b53_masks_a_listed_word_in_small_capitals_as_a_username_refuses_it() {
        for word in [a_token_word(), an_anywhere_word()] {
            let fancy = small_capitals(word);
            assert_ne!(fancy, word, "premise: the word is written in look-alikes");
            assert!(
                normalize_username(&fancy).is_err(),
                "premise: the name is refused"
            );
            assert_eq!(
                mask_blocked_words(&format!("gg {fancy}")),
                format!("gg {}", stars(&fancy))
            );
        }
    }

    #[test]
    fn b53_masks_a_listed_word_with_a_number_beside_it_as_a_username_refuses_it() {
        let word = a_token_word();
        for joined in [format!("{word}1"), format!("1{word}")] {
            assert!(
                normalize_username(&joined).is_err(),
                "premise: {joined} is refused as a name"
            );
            assert_eq!(mask_blocked_words(&joined), stars(&joined));
        }
    }

    #[test]
    fn b53_masks_a_word_exactly_when_the_username_filter_refuses_it() {
        for word in [
            "Scunthorpe",
            "Cassandra",
            "Assassin",
            "Tit4n",
            "Bob455",
            "Bob_455",
        ] {
            let masked = mask_blocked_words(word) != word;
            assert_eq!(masked, normalize_username(word).is_err(), "{word}");
        }
    }
}
