//! The event union (SPEC §10.3). Every visible state change emits one, and BUILD M5-T4 animates each type.
//! Payloads carry ids and numbers only, so an event list serializes and replays exactly.
//!
//! Port of `packages/shared/src/events.ts` (part 1's type freeze, SURFACE §6.4). TS's inline string
//! unions are named enums here (`Position`, `CounterKind`, `Winner`, …); `stolen.zone` and
//! `crumbled.zone` take `ZoneName`, whose literals they are.

use serde::{Deserialize, Serialize};

use crate::wire::catalog_types::{CardType, Keyword, PlayerId, PromptKind, Row, Zone, ZoneName};
use crate::wire::string_union;

string_union! {
    /// A unit's battle position (§4.1). (TS `state.Position`, written inline on the wire.)
    pub enum Position {
        Atk = "ATK",
        Def = "DEF",
    }
}

string_union! {
    /// `cardPlayed.from`: B5 E11, R454 — the card was played from its player's graveyard.
    pub enum PlayedFrom {
        Graveyard = "graveyard",
    }
}

string_union! {
    /// `counterChanged.counter`.
    pub enum CounterKind {
        Plague = "plague",
        Grade = "grade",
        Brittle = "brittle",
    }
}

string_union! {
    /// `rotated.direction`.
    pub enum RotationDirection {
        Left = "left",
        Right = "right",
    }
}

string_union! {
    /// `swapped.what`.
    pub enum SwapWhat {
        Health = "health",
        Board = "board",
        Library = "library",
    }
}

string_union! {
    /// `PlayerId | "draw"`: who won (`gameOver.winner`, `GameState.result.winner`).
    pub enum Winner {
        P1 = "p1",
        P2 = "p2",
        Draw = "draw",
    }
}

impl From<PlayerId> for Winner {
    fn from(player: PlayerId) -> Winner {
        match player {
            PlayerId::P1 => Winner::P1,
            PlayerId::P2 => Winner::P2,
        }
    }
}

impl Winner {
    /// The winning seat, or `None` for a draw.
    pub fn player(self) -> Option<PlayerId> {
        match self {
            Winner::P1 => Some(PlayerId::P1),
            Winner::P2 => Some(PlayerId::P2),
            Winner::Draw => None,
        }
    }
}

string_union! {
    /// `countered.to`: where a countered card went (a steal sends it to a hand, E2).
    pub enum CounteredTo {
        Graveyard = "graveyard",
        Exile = "exile",
        Hand = "hand",
        Gone = "gone",
    }
}

string_union! {
    /// R1366: `controlChanged.how`, the verb that moved the card: §6.3 Steal (`steal`), or R1423's give
    /// to the other player (`give`). Absent for a card a board move carried across (a board swap, a
    /// rotation across the centre line, a rollback's restore).
    pub enum ControlHow {
        Steal = "steal",
        Give = "give",
    }
}

string_union! {
    /// `redirected.what`.
    pub enum RedirectWhat {
        Damage = "damage",
        Attack = "attack",
        Target = "target",
    }
}

string_union! {
    /// R676: Glitch's four outcomes (`glitched.outcome`, config `GLITCH_OUTCOMES`).
    pub enum GlitchOutcome {
        Reset = "reset",
        Swap = "swap",
        Boards = "boards",
        Void = "void",
    }
}

/// `{ winner: PlayerId | "draw"; reason: GameOverReason }`: how a game ended (`GameState.result`,
/// `PlayerView.result`).
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(rename_all = "camelCase")]
pub struct GameResult {
    pub winner: Winner,
    pub reason: GameOverReason,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "type", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum GameEvent {
    /// `formerId` (R227): the id the card had until this moment, set only when the play put it
    /// face-down into a backrow, which gives it a fresh id. Its controller's client finds the hand card
    /// by it; a view that hides the card hides this too (R97).
    CardPlayed {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        cost_paid: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        x: Option<i32>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        embiggened: Option<bool>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        former_id: Option<String>,
        /// B5 E11, R454: the card was played from its player's graveyard, not the hand. Public.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        from: Option<PlayedFrom>,
        /// R119: the permanents that arrived on the field during this play before §10.5 step 4
        /// announced it — a tributed unit's Death at step 2 (#22's copies) — which do not answer it, as
        /// `cardResolved`'s field says for step 7. Engine bookkeeping: a view never forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        arrived_during: Option<Vec<String>>,
        /// R174, R212: the field's departures as the play was announced (the engine's exit mark), so a
        /// response the loop hands this event later judges the played card's stay from the moment the
        /// event happened. Engine bookkeeping: a view never forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        exits_from: Option<u32>,
    },
    /// §10.5 step 7: the card has finished resolving — after its Cry and any Echo repeats, and after a
    /// Spell has reached the graveyard or exile. R17 keys the post-resolution traps on this moment
    /// (Bear Honeypot, Unstable Clone Machine, Unlicensed Experimentation), as against Sheepish, which
    /// fires at step 4 and costs the card its Cry. `permanent` says whether the card is still in play,
    /// which is R61's distinction for Unlicensed Experimentation.
    ///
    /// `costPaid` repeats `cardPlayed`'s number — the mana actually charged after every modifier, R65's
    /// X and embiggen prices included, and 0 for a cast (R70). It is carried rather than looked up
    /// because R89 is the hazard: a trigger answering this event must find what it needs on the event,
    /// since the instance may have been reset between the two moments (#60 reads "costing 1 or less").
    ///
    /// `radiant` is the face that resolved, for the same reason: #33 Unstable Clone Machine copies the
    /// played card with its radiant flag (R34, R57), and by step 7 the card may have ceased to exist —
    /// #41 Sheepish transforms a played Unit at step 4 — so a trigger cannot look it up. It is the
    /// card's flag at step 7 when it still exists, and the flag it was played with otherwise. A view
    /// that hides the card hides this too (R97).
    CardResolved {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        permanent: bool,
        cost_paid: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        radiant: Option<bool>,
        /// R119: the permanents that arrived on the field while this play resolved, whatever put them
        /// there — a Recruit by its Cry (#98), #95's backrow, a Reborn body its own Cry brought back, a
        /// unit a trap answering the play summoned — which do not answer it, as the played card does not
        /// answer its own play. Engine bookkeeping: a view never forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        arrived_during: Option<Vec<String>>,
        /// R174, R212: the field's departures as step 7 emitted this (the engine's exit mark). A cast's
        /// `cardResolved` waits for the loop of the effect that cast it (R70), and the rest of that
        /// effect's list, and the state check after it, can take the card off the field and Reborn put
        /// a new body back before the event reaches a response — which judges the card's stay from
        /// here, not from the dispatch, so that body is not "it". Engine bookkeeping: a view never
        /// forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        exits_from: Option<u32>,
    },
    Summoned {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        row: Row,
        lane: i32,
        /// `formerId` (R227): as on `cardPlayed`, when this summon put an existing card face-down.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        former_id: Option<String>,
        /// R119: on a played card's step-4 `summoned`, as on its `cardPlayed`. A view never forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        arrived_during: Option<Vec<String>>,
        /// R174, R212: on a played card's step-4 `summoned`, as on its `cardPlayed`. A view never forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        exits_from: Option<u32>,
    },
    /// §4.4 step 5: `amount` is what got through. `absorbed` (R1360) is what the target's Armor took
    /// off this hit at step 2 — never the hero's divisors or cap, Divine Shield or Indestructible, and 0
    /// under Pierce — and is left off the wire at 0 (D14), so a hit Armor had no part in serialises as
    /// it did before the field existed. Public, as `amount` is.
    Damage {
        source_id: Option<String>,
        target_id: String,
        amount: i32,
        combat: bool,
        #[serde(default, skip_serializing_if = "is_zero")]
        absorbed: i32,
    },
    HealthLost {
        player: PlayerId,
        amount: i32,
    },
    Healed {
        target_id: String,
        amount: i32,
    },
    DivineShieldLost {
        instance_id: String,
    },
    /// R89: what the card was as it died — the layers' attack and max health, and who killed it.
    Destroyed {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
        /// The player who controlled it as it died. R172: a stolen unit dies as its controller's, though
        /// it goes to its owner's graveyard — Classic #14 Shadowstep's "your Units" reads this.
        controller: PlayerId,
        attack: i32,
        max_health: i32,
        killer_id: Option<String>,
        /// R89: set when the unit died on its Radiant face (C+ #12.8 Frostspatula's memory, R409).
        /// Only ever `Some(true)`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        radiant: Option<bool>,
    },
    EnteredGraveyard {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
    },
    Exiled {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
    },
    Bounced {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
    },
    /// §2.4, R4: a card drawn or added to a full hand, burned on its way in. It is the hand cap's only
    /// event, so it is what "hand full" means on the board (R317). An ordinary card's `enteredGraveyard`
    /// follows it; a unit-token card ceases to exist instead (R11) and has none.
    Burned {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
    },
    /// §2.4, R3, R315: a draw from an empty library that fatigues. `count` is the owner's fatigue count
    /// after this draw (the Nth) and `amount` is the hit it deals before Armor (`FATIGUE_DAMAGE(count)`).
    /// The `damage` instance on the hero follows it, or, when Armor takes the whole hit, the
    /// `damageAbsorbed` that reports it (R240, R1362). A draw #75 Infinite Reserves replaces emits
    /// none. Public: it names no card.
    Fatigue {
        player: PlayerId,
        count: i32,
        amount: i32,
    },
    /// §2.4, R80, R316: a card refused by a full library. `outcome` says what became of it: a card the
    /// effect was creating is `notCreated` (it never existed, #33's and #90's copies), an existing card
    /// goes to its owner's `graveyard` (its `enteredGraveyard` follows), and an existing unit-token card
    /// has `ceased` to exist (R11). `player` owns the library. `instanceId` and `defId` follow R97.
    LibraryOverflow {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        outcome: LibraryOverflowOutcome,
        /// R316: set when the refused card is Radiant (#33's Radiant face makes every copy Radiant, a
        /// Radiant CN-Virus copies Radiant), so the board shows the face it would have had. It is the
        /// card's, so a view that hides the card hides this too (R97). Only ever `Some(true)`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        radiant: Option<bool>,
        /// R316: the card a `notCreated` copy was a copy of (#33 copies whatever its controller plays,
        /// a Trap set face-down included; #90.1 copies itself), so a view judges the copy that was never
        /// made by that card, and a face-down trap's copy does not name it (R97). Engine bookkeeping: a
        /// view never forwards it.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        copy_of: Option<String>,
    },
    Discarded {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
    },
    /// `turnDraw` (B5 E4, R457): this draw's number among `player`'s draws this turn, whoever's turn it
    /// is (1 for the first; a fatigue draw counts, a limited one does not). Public: the hand count and
    /// the fatigue count already say as much. Absent during setup, which is no player's turn (§2.1), so
    /// the opening deal says nothing of which draw a Quickdraw card replaced (R225).
    Drawn {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        turn_draw: Option<i32>,
        /// B5 E33: this draw took the last card of the drawer's own library — Classic #90's quest 9, "a
        /// draw of yours takes the last card of your deck". Present, and `true`, only then. Public: the
        /// library count already says as much.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        emptied: Option<bool>,
    },
    AddedToHand {
        player: PlayerId,
        instance_id: String,
        def_id: String,
    },
    ShuffledIn {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        position: i32,
    },
    Buffed {
        instance_id: String,
        attack: i32,
        health: i32,
    },
    KeywordGranted {
        instance_id: String,
        keyword: Keyword,
        /// R46: set when the unit loses the keyword instead — the Taunt an Indestructible unit's
        /// knock-down takes for the rest of the turn, which a unit already in Attack Position would
        /// otherwise lose with no event (§10.3, R91). Absent on a grant. Only ever `Some(true)`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        lost: Option<bool>,
    },
    /// `brittle` is B3.3's count (R385). `placed` (B5 E19) is how many Plague Counters one placement put
    /// on the card — set on a placement only, so "whenever Plague Counters are placed on this" answers the
    /// placement once however many tokens it placed, and never a removal.
    CounterChanged {
        instance_id: String,
        counter: CounterKind,
        value: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        placed: Option<i32>,
    },
    /// R177: `hiddenFrom` is set on a change made to a card in a library — both players, who could not
    /// read it there (§3) — so a view keeps the event hidden from them for good, even once the card
    /// reads openly. The view uses it and never forwards it.
    CostChanged {
        instance_id: String,
        cost: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hidden_from: Option<Vec<PlayerId>>,
    },
    ModifierChanged {
        player: PlayerId,
        modifier_id: String,
        added: bool,
    },
    RadiantSet {
        instance_id: String,
        def_id: String,
        zone: Zone,
    },
    /// R177: `hiddenFrom` names the players who could not read the old card where it ceased to exist —
    /// both of them for a library card, the other player for a face-down trap (R33) — so a view keeps
    /// it hidden from them for good, even after its replacement reaches a public pile. Absent when the
    /// old card was public. The view uses it and never forwards it.
    Transformed {
        instance_id: String,
        from_def_id: String,
        to_def_id: String,
        new_instance_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hidden_from: Option<Vec<PlayerId>>,
    },
    Fused {
        instance_ids: Vec<String>,
        result_instance_id: String,
        def_id: String,
    },
    PositionSwitched {
        instance_id: String,
        position: Position,
    },
    /// `formerId` (R227): set when the move put the card face-down with a fresh id (C+ #35's restore,
    /// R419), as on `summoned`; a view that hides the card hides this too (R97). `how` (R1366): a steal
    /// or a give, which a board move is neither of; the client's report, which no rule reads
    /// (`as_rules_read`). Public: it says which way the move went, never which card moved.
    ControlChanged {
        instance_id: String,
        controller: PlayerId,
        row: Row,
        lane: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        former_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        how: Option<ControlHow>,
    },
    Rotated {
        direction: RotationDirection,
    },
    Swapped {
        what: SwapWhat,
    },
    Locked {
        player: PlayerId,
        row: Row,
        lane: i32,
    },
    /// R154: `row` and `lane` say which zone flipped, so a client can point at it without being told
    /// which card it was. `instanceId` and `defId` follow §10.8's redaction (R97) — the controller
    /// reads them, the other player reads the sentinel — and a face-down trap is given no instance id
    /// in the view at all, so without the lane the opponent's side has nothing to animate on.
    TrapFired {
        instance_id: String,
        def_id: String,
        controller: PlayerId,
        row: Row,
        lane: i32,
    },
    AttackDeclared {
        attacker_id: String,
        target_id: String,
        forced: bool,
        /// ME-ATTACKSUMMON (R1202): the id of the card whose attack this replaces — Windfast's —
        /// when a summoned substitute makes it. Absent otherwise.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        instead_of: Option<String>,
    },
    AttackCancelled {
        attacker_id: String,
        target_id: String,
        by_instance_id: String,
    },
    ManaChanged {
        player: PlayerId,
        current: i32,
        max: i32,
    },
    TurnStarted {
        player: PlayerId,
        turn: i32,
    },
    TurnEnded {
        player: PlayerId,
        turn: i32,
        unspent_mana: i32,
    },
    TurnAutoEnded {
        player: PlayerId,
        turn: i32,
    },
    PromptOpened {
        player: PlayerId,
        choice_id: String,
        kind: PromptKind,
    },
    PromptAnswered {
        player: PlayerId,
        choice_id: String,
    },
    DrawOffered {
        player: PlayerId,
    },
    DrawAnswered {
        player: PlayerId,
        accept: bool,
    },
    GameOver {
        winner: Winner,
        reason: GameOverReason,
    },
    // -------------------------------------------------------------------------------------------
    // Patch v0.2.0 (docs/classic-sets.md B3, B5). Each has a BUILD M5-T4 row in the client's
    // `ANIMATIONS` and a `SOUND_CUES` row, and follows R97 in `viewFor` like every event above.
    // -------------------------------------------------------------------------------------------
    /// B5 E1: a play or cast has been paid for and is about to move (§10.5 between steps 3 and 4). The
    /// window it opens is where a Counter answers. `cardType` is the type it is played as (a face's own
    /// type, B2.7); `targets` names what the play declared, a hero as `hero-<player>`. A card being set
    /// face-down shows the other player only the zone it is going to (`row`, `lane`), as `cardPlayed`
    /// would (R97, R227).
    CardAnnounced {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        card_type: CardType,
        cost_paid: i32,
        targets: Vec<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        row: Option<Row>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        lane: Option<i32>,
        /// Only ever `Some(true)`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        face_down: Option<bool>,
    },
    /// B5 E1: an announced play was cancelled. `to` is where the card went (a steal sends it to a hand, E2).
    Countered {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        by_instance_id: Option<String>,
        to: CounteredTo,
    },
    /// B5 E2, E16, R466: a card changed owner as it moved to the thief's hand. `zone` is where it was
    /// taken from. A viewer reads the card if they could read it where it was taken from — the hand's
    /// holder, everyone for a public pile or a face-up zone, the controller of a face-down zone, nobody
    /// for a library — or can read it where it is now (R97). `readableFrom` names the first set, written
    /// as the card is taken; the view uses it and never forwards it.
    Stolen {
        instance_id: String,
        def_id: String,
        from: PlayerId,
        to: PlayerId,
        /// One of "hand" | "library" | "resolving" | "graveyard" | "exile" | "field".
        #[cfg_attr(
            feature = "ts",
            ts(type = "\"hand\" | \"library\" | \"resolving\" | \"graveyard\" | \"exile\" | \"field\"")
        )]
        zone: ZoneName,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        readable_from: Option<Vec<PlayerId>>,
    },
    /// B5 E20: a Locked zone opened again.
    Unlocked {
        player: PlayerId,
        row: Row,
        lane: i32,
    },
    /// B3.2, R384: a card's Activate ability was used. `ability` names it (`"activate"` when it has one).
    Activated {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        ability: String,
    },
    /// B3.1, R383: a backrow card stepped into a unit zone as a Unit. `carried` (R446): it was a Unit a
    /// carrier held (Classic+ #33 Ivory Tower), stepping down because its zone no longer carries it — the
    /// same move, from a backrow zone to a unit zone without leaving the field.
    Animated {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        backrow_lane: i32,
        unit_lane: i32,
        /// Only ever `Some(true)`.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional, type = "true"))]
        carried: Option<bool>,
    },
    /// B3.1, R383: an "Animated on your turn" card went back to its backrow zone.
    Deanimated {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        unit_lane: i32,
        backrow_lane: i32,
    },
    /// B3.3, R385, R638: a Brittle count on the field reached 0 and the card was destroyed.
    Crumbled {
        instance_id: String,
        def_id: String,
        owner: PlayerId,
        /// Always "field".
        #[cfg_attr(feature = "ts", ts(type = "\"field\""))]
        zone: ZoneName,
    },
    /// B3.4, R386: one Degrade or Upgrade change. `hiddenFrom` (R177) names the players who could not
    /// read the card where it changed — both for a library card, the other player for a hand card — so a
    /// view keeps it hidden from them for good. The view uses it and never forwards it.
    Degraded {
        instance_id: String,
        def_id: String,
        change: TuningChange,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hidden_from: Option<Vec<PlayerId>>,
    },
    Upgraded {
        instance_id: String,
        def_id: String,
        change: TuningChange,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hidden_from: Option<Vec<PlayerId>>,
    },
    /// B3.4, Classic+ #41 KY's Constant: one of a card's numbers set outright (`key` as `numbersOn` names
    /// it: "cost", "attack", "health", a numbered keyword, a declared number's key). `hiddenFrom` as on
    /// `degraded`: the view uses it and never forwards it.
    NumberChanged {
        instance_id: String,
        def_id: String,
        key: String,
        value: i32,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        #[cfg_attr(feature = "ts", ts(optional))]
        hidden_from: Option<Vec<PlayerId>>,
    },
    /// B5 E9: a damage instance, an attack or a chosen target moved to a new one. Ids as `damage` writes them.
    Redirected {
        what: RedirectWhat,
        from_id: String,
        to_id: String,
        by_instance_id: Option<String>,
    },
    /// B5 E7: a hero's health was set — not damage, not a heal (R18's lose health is the nearest rule).
    HealthSet {
        player: PlayerId,
        health: i32,
        source_id: Option<String>,
    },
    /// Classic #90 (E33): a quest's count moved, or a quest was completed.
    QuestProgressed {
        player: PlayerId,
        instance_id: String,
        quest: String,
        progress: i32,
        goal: i32,
    },
    QuestCompleted {
        player: PlayerId,
        instance_id: String,
        quest: String,
    },
    /// Classic+ #35 (E29): the board went back `turnsAgo` turns on the named sides.
    RolledBack {
        player: PlayerId,
        turns_ago: i32,
        sides: Vec<PlayerId>,
    },
    /// R436: Call to Chaos names the effects it rolled, to both players, in the order they resolve.
    ChaosRolled {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        effects: Vec<String>,
    },
    /// B5 E22: a card left the field and came back into the same zone at once (R78's reset, no Cry, no Death).
    Flickered {
        player: PlayerId,
        instance_id: String,
        def_id: String,
        row: Row,
        lane: i32,
    },
    /// B5 E3: a draw that a draw limit stopped — no card moved, no fatigue. Public: it names no card.
    DrawLimited {
        player: PlayerId,
    },
    /// B5 E10: an effect ended `player`'s turn (the turn's own `turnEnded` follows).
    TurnCutShort {
        player: PlayerId,
        by_instance_id: Option<String>,
    },
    /// R676–R679: a Glitch `player` played did one of its four things. Public, and it names no card:
    /// `reset` (the match starts again, its setup's events follow), `swap` (each account now plays the
    /// other seat; a host reads `GameState.seatSwaps`), `boards` (both fields became other games'
    /// boards) or `void` (the match never happened; `gameOver` with reason `voided` follows).
    Glitched {
        player: PlayerId,
        outcome: GlitchOutcome,
    },
    /// R437: a card gained or lost a mark — a pending effect aimed at it, shown on it in both views
    /// (#50 K-Pop Fanatic's steal is `"steal"`, purple). `color` is a key the client maps to a colour.
    Marked {
        instance_id: String,
        mark: String,
        color: String,
        added: bool,
    },
    /// ME-CN, R1301: a card both players read is shown in Chinese from now on (`effects::translate`).
    /// A card someone cannot read is translated silently (R440), so the id always names a readable card.
    Translated {
        instance_id: String,
    },
    // -------------------------------------------------------------------------------------------
    // Patch v0.3.X (docs/meditative-set.md M8, MN05). Its BUILD M5-T4 row, `ANIMATIONS` and
    // `SOUND_CUES` rows came with it, and `viewFor` redacts it as it does `damage`.
    // -------------------------------------------------------------------------------------------
    /// R1361: a hit the target's Armor took whole at §4.4 step 2 — `absorbed` is the whole hit, after
    /// Spell Damage. It is a report and no damage instance (R63's zero rule): nothing answers it, no
    /// trap, trigger or quest, and no Lifesteal, Poisonous or Trample comes of it. The fields are
    /// `damage`'s, redacted as `damage`'s are (R97). R1362: an absorbed fatigue draw is one, from no
    /// source.
    DamageAbsorbed {
        source_id: Option<String>,
        target_id: String,
        absorbed: i32,
        combat: bool,
    },
    // -------------------------------------------------------------------------------------------
    // Patch v0.3.X (docs/meditative-set.md M5, MB10's ME-JADE). Its BUILD M5-T4 row, `ANIMATIONS`
    // and `SOUND_CUES` rows came with it, and `viewFor` sends it to both seats as it is.
    // -------------------------------------------------------------------------------------------
    /// R961: a player's Jade Counter rose to `value` (`effects::jade`). Public: a player and a number.
    JadeChanged {
        player: PlayerId,
        value: i32,
    },
}

/// serde's `skip_serializing_if` for a number left off the wire at 0 (D14: `damage.absorbed`).
fn is_zero(value: &i32) -> bool {
    *value == 0
}

/// B3.4, R386: what one Degrade or Upgrade application changed. `cost` is a `costMod` step; `stats`
/// the attack and health it moved (negative for a Degrade); `keyword` one keyword added or removed;
/// `x` a numbered keyword's or an X's step (`key` names it: "Armor", "Echo", "Activate", "X", …);
/// `number` a declared number's step (`key` is the catalog `params` key). `delta` is how far the value
/// moved. `none` is R440's cue on a card someone may not read that nothing could change (Immutable, or
/// no change applies), so the events over a hidden pile number the applications, never the changes.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(
    feature = "ts",
    derive(ts_rs::TS),
    ts(export, export_to = "../../../apps/web/src/wire/generated/")
)]
#[serde(tag = "kind", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum TuningChange {
    Cost { delta: i32 },
    Stats { attack: i32, health: i32 },
    Keyword { keyword: Keyword, added: bool },
    X { key: String, delta: i32 },
    Number { key: String, delta: i32 },
    None,
}

string_union! {
    /// `GameEvent["type"]`, in `GAME_EVENT_TYPES` order.
    pub enum GameEventType {
        CardPlayed = "cardPlayed",
        CardResolved = "cardResolved",
        Summoned = "summoned",
        Damage = "damage",
        HealthLost = "healthLost",
        Healed = "healed",
        DivineShieldLost = "divineShieldLost",
        Destroyed = "destroyed",
        EnteredGraveyard = "enteredGraveyard",
        Exiled = "exiled",
        Bounced = "bounced",
        Burned = "burned",
        Fatigue = "fatigue",
        LibraryOverflow = "libraryOverflow",
        Discarded = "discarded",
        Drawn = "drawn",
        AddedToHand = "addedToHand",
        ShuffledIn = "shuffledIn",
        Buffed = "buffed",
        KeywordGranted = "keywordGranted",
        CounterChanged = "counterChanged",
        CostChanged = "costChanged",
        ModifierChanged = "modifierChanged",
        RadiantSet = "radiantSet",
        Transformed = "transformed",
        Fused = "fused",
        PositionSwitched = "positionSwitched",
        ControlChanged = "controlChanged",
        Rotated = "rotated",
        Swapped = "swapped",
        Locked = "locked",
        TrapFired = "trapFired",
        AttackDeclared = "attackDeclared",
        AttackCancelled = "attackCancelled",
        ManaChanged = "manaChanged",
        TurnStarted = "turnStarted",
        TurnEnded = "turnEnded",
        TurnAutoEnded = "turnAutoEnded",
        PromptOpened = "promptOpened",
        PromptAnswered = "promptAnswered",
        DrawOffered = "drawOffered",
        DrawAnswered = "drawAnswered",
        GameOver = "gameOver",
        CardAnnounced = "cardAnnounced",
        Countered = "countered",
        Stolen = "stolen",
        Unlocked = "unlocked",
        Activated = "activated",
        Animated = "animated",
        Deanimated = "deanimated",
        Crumbled = "crumbled",
        Degraded = "degraded",
        Upgraded = "upgraded",
        NumberChanged = "numberChanged",
        Redirected = "redirected",
        HealthSet = "healthSet",
        QuestProgressed = "questProgressed",
        QuestCompleted = "questCompleted",
        RolledBack = "rolledBack",
        ChaosRolled = "chaosRolled",
        Flickered = "flickered",
        DrawLimited = "drawLimited",
        TurnCutShort = "turnCutShort",
        Marked = "marked",
        Glitched = "glitched",
        Translated = "translated",
        DamageAbsorbed = "damageAbsorbed",
        JadeChanged = "jadeChanged",
    }
}

/// Every type in the union, for the animation-table test (BUILD M5-T4). In Rust the union and this
/// list cannot drift apart: `GameEvent::event_type` is an exhaustive match.
pub const GAME_EVENT_TYPES: &[GameEventType] = GameEventType::ALL;

impl GameEvent {
    /// The event as the rules read it, which is how the trigger loop holds it in `state.dispatch` and
    /// hands it to the traps, the triggers and the quests. R1360: a `damage` without its `absorbed`,
    /// which reports what Armor took to the client (§10.10, §10.11) and which no rule reads. R1361: a
    /// `damageAbsorbed` as the hit of 0 it is to the rules (R63), which nothing answers — R240's report
    /// of an absorbed fatigue draw, the one that reaches the loop (R1362). R1366: a `controlChanged`
    /// without its `how`, likewise the client's. So the state a game passes through, a frontier paused
    /// on a prompt or left by a game's end included, hashes as it did before any of them existed (D14),
    /// and a recorded game's replay keeps its final hash (R768). Every other event is itself.
    pub fn as_rules_read(&self) -> GameEvent {
        match self {
            GameEvent::Damage {
                source_id,
                target_id,
                amount,
                combat,
                absorbed: _,
            } => GameEvent::Damage {
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                amount: *amount,
                combat: *combat,
                absorbed: 0,
            },
            GameEvent::DamageAbsorbed {
                source_id,
                target_id,
                combat,
                absorbed: _,
            } => GameEvent::Damage {
                source_id: source_id.clone(),
                target_id: target_id.clone(),
                amount: 0,
                combat: *combat,
                absorbed: 0,
            },
            GameEvent::ControlChanged {
                instance_id,
                controller,
                row,
                lane,
                former_id,
                how: _,
            } => GameEvent::ControlChanged {
                instance_id: instance_id.clone(),
                controller: *controller,
                row: *row,
                lane: *lane,
                former_id: former_id.clone(),
                how: None,
            },
            other => other.clone(),
        }
    }

    /// `event.type`.
    pub fn event_type(&self) -> GameEventType {
        match self {
            GameEvent::CardPlayed { .. } => GameEventType::CardPlayed,
            GameEvent::CardResolved { .. } => GameEventType::CardResolved,
            GameEvent::Summoned { .. } => GameEventType::Summoned,
            GameEvent::Damage { .. } => GameEventType::Damage,
            GameEvent::HealthLost { .. } => GameEventType::HealthLost,
            GameEvent::Healed { .. } => GameEventType::Healed,
            GameEvent::DivineShieldLost { .. } => GameEventType::DivineShieldLost,
            GameEvent::Destroyed { .. } => GameEventType::Destroyed,
            GameEvent::EnteredGraveyard { .. } => GameEventType::EnteredGraveyard,
            GameEvent::Exiled { .. } => GameEventType::Exiled,
            GameEvent::Bounced { .. } => GameEventType::Bounced,
            GameEvent::Burned { .. } => GameEventType::Burned,
            GameEvent::Fatigue { .. } => GameEventType::Fatigue,
            GameEvent::LibraryOverflow { .. } => GameEventType::LibraryOverflow,
            GameEvent::Discarded { .. } => GameEventType::Discarded,
            GameEvent::Drawn { .. } => GameEventType::Drawn,
            GameEvent::AddedToHand { .. } => GameEventType::AddedToHand,
            GameEvent::ShuffledIn { .. } => GameEventType::ShuffledIn,
            GameEvent::Buffed { .. } => GameEventType::Buffed,
            GameEvent::KeywordGranted { .. } => GameEventType::KeywordGranted,
            GameEvent::CounterChanged { .. } => GameEventType::CounterChanged,
            GameEvent::CostChanged { .. } => GameEventType::CostChanged,
            GameEvent::ModifierChanged { .. } => GameEventType::ModifierChanged,
            GameEvent::RadiantSet { .. } => GameEventType::RadiantSet,
            GameEvent::Transformed { .. } => GameEventType::Transformed,
            GameEvent::Fused { .. } => GameEventType::Fused,
            GameEvent::PositionSwitched { .. } => GameEventType::PositionSwitched,
            GameEvent::ControlChanged { .. } => GameEventType::ControlChanged,
            GameEvent::Rotated { .. } => GameEventType::Rotated,
            GameEvent::Swapped { .. } => GameEventType::Swapped,
            GameEvent::Locked { .. } => GameEventType::Locked,
            GameEvent::TrapFired { .. } => GameEventType::TrapFired,
            GameEvent::AttackDeclared { .. } => GameEventType::AttackDeclared,
            GameEvent::AttackCancelled { .. } => GameEventType::AttackCancelled,
            GameEvent::ManaChanged { .. } => GameEventType::ManaChanged,
            GameEvent::TurnStarted { .. } => GameEventType::TurnStarted,
            GameEvent::TurnEnded { .. } => GameEventType::TurnEnded,
            GameEvent::TurnAutoEnded { .. } => GameEventType::TurnAutoEnded,
            GameEvent::PromptOpened { .. } => GameEventType::PromptOpened,
            GameEvent::PromptAnswered { .. } => GameEventType::PromptAnswered,
            GameEvent::DrawOffered { .. } => GameEventType::DrawOffered,
            GameEvent::DrawAnswered { .. } => GameEventType::DrawAnswered,
            GameEvent::GameOver { .. } => GameEventType::GameOver,
            GameEvent::CardAnnounced { .. } => GameEventType::CardAnnounced,
            GameEvent::Countered { .. } => GameEventType::Countered,
            GameEvent::Stolen { .. } => GameEventType::Stolen,
            GameEvent::Unlocked { .. } => GameEventType::Unlocked,
            GameEvent::Activated { .. } => GameEventType::Activated,
            GameEvent::Animated { .. } => GameEventType::Animated,
            GameEvent::Deanimated { .. } => GameEventType::Deanimated,
            GameEvent::Crumbled { .. } => GameEventType::Crumbled,
            GameEvent::Degraded { .. } => GameEventType::Degraded,
            GameEvent::Upgraded { .. } => GameEventType::Upgraded,
            GameEvent::NumberChanged { .. } => GameEventType::NumberChanged,
            GameEvent::Redirected { .. } => GameEventType::Redirected,
            GameEvent::HealthSet { .. } => GameEventType::HealthSet,
            GameEvent::QuestProgressed { .. } => GameEventType::QuestProgressed,
            GameEvent::QuestCompleted { .. } => GameEventType::QuestCompleted,
            GameEvent::RolledBack { .. } => GameEventType::RolledBack,
            GameEvent::ChaosRolled { .. } => GameEventType::ChaosRolled,
            GameEvent::Flickered { .. } => GameEventType::Flickered,
            GameEvent::DrawLimited { .. } => GameEventType::DrawLimited,
            GameEvent::TurnCutShort { .. } => GameEventType::TurnCutShort,
            GameEvent::Glitched { .. } => GameEventType::Glitched,
            GameEvent::Marked { .. } => GameEventType::Marked,
            GameEvent::Translated { .. } => GameEventType::Translated,
            GameEvent::DamageAbsorbed { .. } => GameEventType::DamageAbsorbed,
            GameEvent::JadeChanged { .. } => GameEventType::JadeChanged,
        }
    }
}

string_union! {
    /// R316: what became of a card R80's full library refused (`libraryOverflow.outcome`).
    pub enum LibraryOverflowOutcome {
        NotCreated = "notCreated",
        Graveyard = "graveyard",
        Ceased = "ceased",
    }
}

string_union! {
    /// How a game ended (`gameOver.reason`).
    pub enum GameOverReason {
        HeroDeath = "hero-death",
        BothHeroesDead = "both-heroes-dead",
        Concede = "concede",
        DrawAccepted = "draw-accepted",
        TurnCap = "turn-cap",
        Disconnect = "disconnect",
        MatchCeiling = "match-ceiling",
        /// R679: a Glitch voided the match — no winner, no result, no record (§2.5).
        Voided = "voided",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_serialise_as_ts_writes_them() {
        let event = GameEvent::Damage {
            source_id: None,
            target_id: "hero-p2".into(),
            amount: 3,
            combat: false,
            absorbed: 0,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(
            json,
            r#"{"type":"damage","sourceId":null,"targetId":"hero-p2","amount":3,"combat":false}"#
        );
        assert_eq!(serde_json::from_str::<GameEvent>(&json).unwrap(), event);
        let over = GameEvent::GameOver {
            winner: Winner::Draw,
            reason: GameOverReason::TurnCap,
        };
        assert_eq!(
            serde_json::to_string(&over).unwrap(),
            r#"{"type":"gameOver","winner":"draw","reason":"turn-cap"}"#
        );
        assert_eq!(over.event_type().as_str(), "gameOver");
        assert_eq!(GAME_EVENT_TYPES.len(), 68);
    }

    /// R1360, D14: `absorbed` is on the wire only when Armor took part of the hit, so a hit it had no
    /// part in reads exactly as before (the test above), and a line without it reads back as 0.
    #[test]
    fn r1360_absorbed_rides_the_damage_event_only_when_armor_took_part() {
        let event = GameEvent::Damage {
            source_id: Some("c4".into()),
            target_id: "c9".into(),
            amount: 1,
            combat: true,
            absorbed: 2,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(
            json,
            r#"{"type":"damage","sourceId":"c4","targetId":"c9","amount":1,"combat":true,"absorbed":2}"#
        );
        assert_eq!(serde_json::from_str::<GameEvent>(&json).unwrap(), event);
        let old = r#"{"type":"damage","sourceId":null,"targetId":"hero-p1","amount":4,"combat":false}"#;
        assert!(matches!(
            serde_json::from_str::<GameEvent>(old).unwrap(),
            GameEvent::Damage { absorbed: 0, .. }
        ));
    }

    /// R1361: the report of a hit Armor took whole has `damage`'s fields, `absorbed` for `amount`.
    #[test]
    fn r1361_damage_absorbed_serialises_with_damages_fields() {
        let event = GameEvent::DamageAbsorbed {
            source_id: None,
            target_id: "hero-p2".into(),
            absorbed: 3,
            combat: false,
        };
        let json = serde_json::to_string(&event).unwrap();
        assert_eq!(
            json,
            r#"{"type":"damageAbsorbed","sourceId":null,"targetId":"hero-p2","absorbed":3,"combat":false}"#
        );
        assert_eq!(serde_json::from_str::<GameEvent>(&json).unwrap(), event);
        assert_eq!(event.event_type().as_str(), "damageAbsorbed");
        assert_eq!(GAME_EVENT_TYPES.last(), Some(&GameEventType::DamageAbsorbed));
    }
}
