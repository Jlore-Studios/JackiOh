//! What the log may call a card, a hero or a seat (SPEC §10.10, R97): every name is read off the event's
//! payload and the public board the view carries, so a card the viewer may not read is never named. The
//! sentinel (`hidden`) names no card, and an id is never printed.

use indexmap::IndexMap;
use jackioh_engine::wire::{
    BackrowView, CardDef, GameEvent, HandView, PlayerId, PlayerView, QuestView, SideView,
};

use crate::face::chinese::{chinese_name, is_glitch};
use crate::face::glitch::glitch_words;
use crate::face::live::{HIDDEN_CARD, NamedCard};
use crate::face::types::DefMap;
use crate::face::words::Words;

/// A card the view lists face up, as far as a name reads it.
pub struct Seen<'a> {
    pub def_id: &'a str,
    pub radiant: bool,
    /// R1301: the view says the card is shown in Chinese.
    pub chinese: bool,
    pub quest: Option<&'a QuestView>,
}

/// The instance the view lists face up anywhere a card is public: the field, the piles (the resolving pile
/// among them when `resolving`) and the viewer's own hand. None when the view shows it nowhere, which is also
/// what a hidden card gives.
pub fn find_card<'a>(view: &'a PlayerView, instance_id: &str, resolving: bool) -> Option<Seen<'a>> {
    if instance_id == HIDDEN_CARD {
        return None;
    }
    for side in [&view.you, &view.opponent] {
        for unit in side.units.iter().flatten() {
            if unit.instance_id == instance_id {
                return Some(Seen {
                    def_id: &unit.def_id,
                    radiant: unit.radiant,
                    chinese: unit.chinese == Some(true),
                    quest: unit.quest.as_ref(),
                });
            }
        }
        for entry in side.backrow.iter().flatten() {
            if let BackrowView::Public(card) = entry
                && card.instance_id == instance_id
            {
                return Some(Seen {
                    def_id: &card.def_id,
                    radiant: card.radiant,
                    chinese: card.chinese == Some(true),
                    quest: card.quest.as_ref(),
                });
            }
        }
        let piles: &[&Vec<_>] = if resolving {
            &[&side.graveyard, &side.exile, &side.resolving]
        } else {
            &[&side.graveyard, &side.exile]
        };
        let hand: &[_] = match &side.hand {
            HandView::Cards(cards) => cards,
            HandView::Count { .. } => &[],
        };
        for card in piles.iter().flat_map(|pile| pile.iter()).chain(hand) {
            if card.instance_id == instance_id {
                return Some(Seen {
                    def_id: &card.def_id,
                    radiant: card.radiant,
                    chinese: card.chinese == Some(true),
                    quest: card.quest.as_ref(),
                });
            }
        }
    }
    None
}

/// Instance to definition, from the window's events about public zones, so a unit that has since left the
/// board is still named. The hand's own events (`drawn`, `addedToHand`, `shuffledIn`) are not read, and
/// neither is the sentinel.
pub fn public_names(events: &[GameEvent]) -> IndexMap<String, String> {
    let mut names = IndexMap::new();
    let mut note = |instance_id: &str, def_id: &str| {
        if instance_id != HIDDEN_CARD && def_id != HIDDEN_CARD {
            names.insert(instance_id.to_string(), def_id.to_string());
        }
    };
    for event in events {
        match event {
            GameEvent::CardPlayed {
                instance_id, def_id, ..
            }
            | GameEvent::Summoned {
                instance_id, def_id, ..
            }
            | GameEvent::Destroyed {
                instance_id, def_id, ..
            }
            | GameEvent::EnteredGraveyard {
                instance_id, def_id, ..
            }
            | GameEvent::Exiled {
                instance_id, def_id, ..
            }
            | GameEvent::Bounced {
                instance_id, def_id, ..
            }
            | GameEvent::Burned {
                instance_id, def_id, ..
            }
            | GameEvent::LibraryOverflow {
                instance_id, def_id, ..
            }
            | GameEvent::Discarded {
                instance_id, def_id, ..
            }
            | GameEvent::RadiantSet {
                instance_id, def_id, ..
            }
            | GameEvent::Deradianted {
                instance_id, def_id, ..
            }
            | GameEvent::TrapFired {
                instance_id, def_id, ..
            }
            | GameEvent::Held {
                instance_id, def_id, ..
            } => note(instance_id, def_id),
            GameEvent::Fused {
                result_instance_id,
                def_id,
                ..
            } => note(result_instance_id, def_id),
            GameEvent::Transformed {
                instance_id,
                to_def_id,
                ..
            } => note(instance_id, to_def_id),
            _ => {}
        }
    }
    names
}

/// For each `cardPlayed` of the window, the targets its own announce declared. Paired by the play itself (its
/// index), so a card played twice in one window pairs each play with its own. A face-down set (the other seat
/// reads its targets as the sentinel, and its play takes a fresh id, R97, R227) and a hidden announce pair
/// with nothing, nor does a play whose announce has left the window.
pub fn announced_aims(events: &[GameEvent]) -> Vec<Option<&[String]>> {
    let mut declared: IndexMap<&str, &[String]> = IndexMap::new();
    let mut aims = vec![None; events.len()];
    for (at, event) in events.iter().enumerate() {
        match event {
            GameEvent::CardAnnounced {
                instance_id,
                targets,
                face_down,
                ..
            } => {
                if instance_id != HIDDEN_CARD && *face_down != Some(true) {
                    declared.insert(instance_id, targets);
                }
            }
            GameEvent::CardPlayed { instance_id, .. } => {
                aims[at] = declared.shift_remove(instance_id.as_str());
            }
            _ => {}
        }
    }
    aims
}

/// What the lines of one view call a thing in the reader's words.
pub struct Names<'a> {
    pub view: &'a PlayerView,
    pub words: Words,
    defs: &'a DefMap,
    remembered: IndexMap<String, String>,
}

impl<'a> Names<'a> {
    pub fn new(view: &'a PlayerView, words: Words, defs: &'a DefMap) -> Names<'a> {
        Names {
            view,
            words,
            defs,
            remembered: public_names(&view.events),
        }
    }

    /// The definition `def_id` names: the catalog's, else the match's own (R243).
    fn definition(&self, def_id: &str) -> Option<&'a CardDef> {
        jackioh_engine::catalog::find_def(None, def_id).or_else(|| self.defs.get(def_id))
    }

    /// A card's printed name, or its def id when nothing names it; Glitch's, corrupted as a lookup answers for
    /// it (`face/glitch.rs`).
    fn english(&self, def_id: &str) -> (String, Option<&'a CardDef>) {
        let def = self.definition(def_id);
        let name = if is_glitch(def_id) {
            glitch_words().name
        } else {
            def.map_or_else(|| def_id.to_string(), |def| def.name.clone())
        };
        (name, def)
    }

    /// `a_card` words: the reader's.
    pub fn say(&self, english: &str) -> String {
        self.words.say(english)
    }

    /// A card by its printed name, in Chinese for a reader of Chinese (R711000).
    pub fn def(&self, def_id: &str) -> String {
        if def_id == HIDDEN_CARD {
            return self.say("a hidden card");
        }
        let (english, def) = self.english(def_id);
        if self.words.drawn_in_chinese(false) {
            chinese_name(def_id, &english, def)
        } else {
            english
        }
    }

    /// As `def` names it, but in Chinese while the view shows that instance in Chinese (R1301), so the lines
    /// about one card name it as the board does, whichever field the event carries (#552).
    pub fn card(&self, instance_id: &str, def_id: &str) -> String {
        if def_id == HIDDEN_CARD || instance_id == HIDDEN_CARD {
            return self.def(def_id);
        }
        let flagged = find_card(self.view, instance_id, true).is_some_and(|seen| seen.chinese);
        if !self.words.drawn_in_chinese(flagged) {
            return self.def(def_id);
        }
        let (english, def) = self.english(def_id);
        chinese_name(def_id, &english, def)
    }

    /// The definition of an instance the view lists or the window named, if any.
    pub fn def_id_of(&self, instance_id: &str) -> Option<String> {
        if let Some(seen) = find_card(self.view, instance_id, false) {
            return Some(seen.def_id.to_string());
        }
        self.remembered.get(instance_id).cloned()
    }

    /// A card as `instance` names it, a hero as a seat's; None where nothing public names it (R97: never the
    /// sentinel).
    pub fn known(&self, instance_id: &str) -> Option<String> {
        if instance_id == format!("hero-{}", self.view.you.player) {
            return Some(self.say("your hero"));
        }
        if instance_id == format!("hero-{}", self.view.opponent.player) {
            return Some(self.say("the opponent's hero"));
        }
        if instance_id == HIDDEN_CARD {
            return None;
        }
        let def_id = self.def_id_of(instance_id)?;
        let (english, def) = self.english(&def_id);
        // ME-CN, R1301, R711000: a card the view shows in Chinese is named in Chinese.
        let flagged = find_card(self.view, instance_id, true).is_some_and(|seen| seen.chinese);
        Some(if self.words.drawn_in_chinese(flagged) {
            chinese_name(&def_id, &english, def)
        } else {
            english
        })
    }

    /// A card by name where it is public, else `unknown` (or "a unit"). Heroes read as a seat's.
    pub fn instance(&self, instance_id: &str, unknown: Option<&str>) -> String {
        self.known(instance_id)
            .unwrap_or_else(|| self.say(unknown.unwrap_or("a unit")))
    }

    /// A card an overflow names: by name where the viewer reads it, else "a card" (R97).
    pub fn named(&self, def_id: &str) -> String {
        if def_id == HIDDEN_CARD {
            self.say("a card")
        } else {
            self.def(def_id)
        }
    }

    /// The same, naming the card as the board shows that instance (R1301's Chinese name included).
    pub fn named_card(&self, instance_id: &str, def_id: &str) -> String {
        if def_id == HIDDEN_CARD {
            self.say("a card")
        } else {
            self.card(instance_id, def_id)
        }
    }

    /// A seat, as the subject of a line.
    pub fn seat(&self, player: PlayerId) -> String {
        if self.is_you(player) {
            self.say("You")
        } else {
            self.say("Opponent")
        }
    }

    /// A seat, as the owner of what the line is about.
    pub fn whose(&self, player: PlayerId) -> String {
        if self.is_you(player) {
            self.say("your")
        } else {
            self.say("the opponent's")
        }
    }

    /// R1843000: the player a vote would remove, as the object of the line.
    pub fn removed(&self, player: PlayerId) -> String {
        if self.is_you(player) {
            self.say("you")
        } else {
            self.say("your opponent")
        }
    }

    pub fn is_you(&self, player: PlayerId) -> bool {
        player == self.view.viewer
    }

    /// R370: the cost the view gives the face-down card in `player`'s backrow lane `lane`, while one stands
    /// there.
    pub fn face_down_cost(&self, player: PlayerId, lane: i32) -> Option<i32> {
        let side: &SideView = if self.is_you(player) {
            &self.view.you
        } else {
            &self.view.opponent
        };
        let at = usize::try_from(lane.checked_sub(1)?).ok()?;
        match side.backrow.get(at)? {
            Some(BackrowView::FaceDown(card)) => card.cost,
            _ => None,
        }
    }

    /// The card the line about `event` opens, if it names one: it reads the same sources the names do, so it
    /// can never open a card the line does not print (R97: the sentinel is none).
    pub fn card_of(&self, event: &GameEvent) -> Option<NamedCard> {
        let by_def = |def_id: &str, instance_id: Option<&str>| -> Option<NamedCard> {
            if def_id == HIDDEN_CARD {
                return None;
            }
            let radiant = instance_id
                .and_then(|id| find_card(self.view, id, true))
                .is_some_and(|seen| seen.radiant);
            Some(NamedCard {
                def_id: def_id.to_string(),
                radiant,
                instance_id: instance_id.filter(|id| *id != HIDDEN_CARD).map(str::to_string),
                cost: None,
            })
        };
        let by_instance = |instance_id: &str| -> Option<NamedCard> {
            if instance_id == HIDDEN_CARD {
                return None;
            }
            let def_id = self.def_id_of(instance_id)?;
            by_def(&def_id, Some(instance_id))
        };
        match event {
            GameEvent::CardPlayed {
                instance_id, def_id, ..
            }
            | GameEvent::Summoned {
                instance_id, def_id, ..
            }
            | GameEvent::Destroyed {
                instance_id, def_id, ..
            }
            | GameEvent::EnteredGraveyard {
                instance_id, def_id, ..
            }
            | GameEvent::Exiled {
                instance_id, def_id, ..
            }
            | GameEvent::Bounced {
                instance_id, def_id, ..
            }
            | GameEvent::Burned {
                instance_id, def_id, ..
            }
            | GameEvent::LibraryOverflow {
                instance_id, def_id, ..
            }
            | GameEvent::Discarded {
                instance_id, def_id, ..
            }
            | GameEvent::TrapFired {
                instance_id, def_id, ..
            } => {
                if instance_id == HIDDEN_CARD {
                    None
                } else {
                    by_def(def_id, Some(instance_id))
                }
            }
            GameEvent::RadiantSet {
                instance_id, def_id, ..
            } => {
                if instance_id == HIDDEN_CARD {
                    return None;
                }
                by_def(def_id, Some(instance_id)).map(|card| NamedCard {
                    radiant: true,
                    ..card
                })
            }
            GameEvent::Deradianted {
                instance_id, def_id, ..
            } => {
                if instance_id == HIDDEN_CARD {
                    return None;
                }
                by_def(def_id, Some(instance_id)).map(|card| NamedCard {
                    radiant: false,
                    ..card
                })
            }
            GameEvent::Transformed {
                to_def_id,
                new_instance_id,
                ..
            } => by_def(to_def_id, Some(new_instance_id)),
            GameEvent::Fused {
                def_id,
                result_instance_id,
                ..
            } => by_def(def_id, Some(result_instance_id)),
            GameEvent::Damage { target_id, .. }
            | GameEvent::DamageAbsorbed { target_id, .. }
            | GameEvent::Healed { target_id, .. } => by_instance(target_id),
            GameEvent::DivineShieldLost { instance_id }
            | GameEvent::Buffed { instance_id, .. }
            | GameEvent::KeywordGranted { instance_id, .. }
            | GameEvent::CounterChanged { instance_id, .. }
            | GameEvent::CostChanged { instance_id, .. }
            | GameEvent::PositionSwitched { instance_id, .. }
            | GameEvent::ControlChanged { instance_id, .. }
            | GameEvent::QuestProgressed { instance_id, .. }
            | GameEvent::QuestCompleted { instance_id, .. } => by_instance(instance_id),
            GameEvent::AttackDeclared { attacker_id, .. }
            | GameEvent::AttackCancelled { attacker_id, .. } => by_instance(attacker_id),
            _ => None,
        }
    }
}
