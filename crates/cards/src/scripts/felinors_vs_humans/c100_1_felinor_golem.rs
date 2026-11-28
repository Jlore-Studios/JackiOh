//! FvH #100.1 Felinor Golem (SPEC §8.14 row 100.1; R5237010, R5238008, R5262003): (4) Unit, Token (printed
//! Legendary), Felinor, Golem, 5/5 → 10/10, of the held Felinors vs Humans set. Its maker is FvH #100 Call to
//! Chaos (Felinors vs. Humans Edition), entry 9.
//!   Base:    "Rush, Reborn / Immune to Humans. / Allure every Unit this is in combat with."
//!   Radiant: "Rush, Cleave, Reborn / Immune to Humans. / Allure every Unit this is in combat with."
//! Engine: Rush, Reborn and the Radiant face's Cleave are catalog data. Immune to Humans is
//! `StaticFlags.immune_to_tag` Human (R5237010): a card whose tags hold Human cannot target, affect or damage
//! it, a Human attacker's combat damage included; it bars no attack. R5262003: "every Unit this is in combat
//! with" is the Unit it attacks (`when_attacks`, before any damage) and the Unit that attacks it
//! (`after_attacked`), never a Cleave splash's side victims; each is Allured by `allure_unit` naming it
//! (R5238008), so it is stolen at the start of this card's controller's next turn, a card gone first not
//! taken (R174). A hero is no Unit and is not Allured.

use jackioh_engine::prelude::*;

pub const ID: &str = "felinorsvshumans-100-1";

/// R5262003, R5238008: Allure the Unit `instance_id` names; an id naming no enemy Unit (a hero) Allures none.
fn allure(instance_id: String) -> Vec<Effect> {
    vec![allure_unit(AllureUnitArgs {
        instance_id: Some(instance_id),
        random_enemy: None,
    })]
}

fn golem() -> Script {
    Script {
        // R5237010: Immune to Humans.
        static_flags: Some(StaticFlags {
            immune_to_tag: Some(Tag::Human),
            ..StaticFlags::default()
        }),
        // R5262003: the Unit it attacks.
        when_attacks: Some(hook(|ctx| {
            when_attacks_of(ctx).map(|facts| allure(facts.target_id)).unwrap_or_default()
        })),
        // R5262003: the Unit that attacks it.
        after_attacked: Some(hook(|ctx| {
            after_attacked_of(ctx).map(|facts| allure(facts.attacker_id)).unwrap_or_default()
        })),
        ..Script::default()
    }
}

pub fn script() -> CardScripts {
    // The Radiant face adds Cleave and doubles the stats: catalog data only.
    let base = golem();
    let radiant = base.clone();
    CardScripts { base, radiant }
}

// FvH #100.1 Felinor Golem — SPEC §8.14 row 100.1, R5237010, R5262003.
#[cfg(test)]
mod tests {
    use jackioh_engine::effects::ALLURE_MARK;
    use jackioh_engine::marks::marks_on;
    use jackioh_engine::testkit::PlayerId::{P1, P2};
    use jackioh_engine::testkit::*;

    use super::ID;
    use crate::{card_def, filler, next_turn_of, scenario};

    const FILLER: &str = "core-005"; // Stockpile: a card in hand, so no turn ends by itself
    /// A sturdy Unit that strikes back; a test gives it a tag.
    const WALL: &str = "stand-in (1) 3/9";
    /// The Golem's printed Health, and the Radiant face's.
    const HEALTH: i32 = 5;
    const RADIANT_HEALTH: i32 = 10;

    /// p1 holds the Golem in lane 2 on `radiant`'s face; p2 `defenders` in lanes 1 to 3; `active` moves first.
    fn board(radiant: bool, defenders: Value, active: &str) -> Scenario {
        let mut s = scenario(json!({
            "active": active,
            "p1": { "hand": [FILLER], "field": [{ "def": ID, "lane": 2, "radiant": radiant }], "library": filler(6) },
            "p2": { "hand": [FILLER], "field": defenders, "library": filler(6) },
        }));
        for player in [P1, P2] {
            s.state_mut().players[player].auto_end_turn = Some(false);
        }
        s
    }

    fn tag(s: &mut Scenario, card: &CardInstance, tag: Tag) {
        s.card_mut(card.id.as_str()).granted_tags = Some(vec![tag]);
    }

    fn allured(s: &Scenario, card: &CardInstance) -> bool {
        marks_on(s.state(), &card.id).iter().any(|mark| mark.mark == ALLURE_MARK)
    }

    fn health_of(s: &Scenario, card: &CardInstance) -> i32 {
        s.stats(card.id.as_str()).health
    }

    #[test]
    fn its_card_declares_no_numbers() {
        crate::register_all();
        assert!(card_def(ID).params.is_none());
    }

    mod base {
        use super::*;

        #[test]
        fn r5237010_a_human_it_attacks_deals_it_nothing_back() {
            let mut s = board(false, json!([{ "def": WALL, "lane": 2 }]), "p1");
            let golem = s.unit_at(P1, 2);
            let human = s.unit_at(P2, 2);
            tag(&mut s, &human, Tag::Human);
            s.attack(&golem, &human);
            assert_eq!(health_of(&s, &golem), HEALTH, "Immune to Humans");
            assert!(health_of(&s, &human) < 9, "the Golem's hit lands");
        }

        #[test]
        fn r5237010_a_unit_of_another_tag_still_hurts_it() {
            let mut s = board(false, json!([{ "def": WALL, "lane": 2 }]), "p1");
            let golem = s.unit_at(P1, 2);
            let felinor = s.unit_at(P2, 2);
            tag(&mut s, &felinor, Tag::Felinor);
            s.attack(&golem, &felinor);
            assert_eq!(health_of(&s, &golem), HEALTH - 3);
        }

        #[test]
        fn r5237010_a_human_may_still_attack_it_and_takes_its_strike_back() {
            let mut s = board(false, json!([{ "def": WALL, "lane": 2 }]), "p2");
            let golem = s.unit_at(P1, 2);
            let human = s.unit_at(P2, 2);
            tag(&mut s, &human, Tag::Human);
            s.attack(&human, &golem);
            assert_eq!(health_of(&s, &golem), HEALTH);
            assert_eq!(health_of(&s, &human), 9 - HEALTH);
        }

        #[test]
        fn r5262003_the_unit_it_attacks_is_allured_and_stolen_at_your_next_turn() {
            let mut s = board(false, json!([{ "def": WALL, "lane": 2 }]), "p1");
            let golem = s.unit_at(P1, 2);
            let target = s.unit_at(P2, 2);
            s.attack(&golem, &target);
            assert!(allured(&s, &target));
            next_turn_of(&mut s, P1);
            assert_eq!(s.card(target.id.as_str()).controller, P1);
            assert!(!allured(&s, &target));
        }

        #[test]
        fn r5262003_the_unit_that_attacks_it_is_allured() {
            let mut s = board(false, json!([{ "def": WALL, "lane": 2 }]), "p2");
            let golem = s.unit_at(P1, 2);
            let attacker = s.unit_at(P2, 2);
            s.attack(&attacker, &golem);
            assert!(allured(&s, &attacker));
            next_turn_of(&mut s, P1);
            assert_eq!(s.card(attacker.id.as_str()).controller, P1);
        }
    }

    mod radiant {
        use super::*;

        #[test]
        fn r5237010_a_human_it_attacks_deals_it_nothing_back() {
            let mut s = board(true, json!([{ "def": WALL, "lane": 2 }]), "p1");
            let golem = s.unit_at(P1, 2);
            let human = s.unit_at(P2, 2);
            tag(&mut s, &human, Tag::Human);
            s.attack(&golem, &human);
            assert_eq!(health_of(&s, &golem), RADIANT_HEALTH);
        }

        #[test]
        fn r5262003_cleave_s_side_victims_are_not_allured() {
            let mut s = board(
                true,
                json!([
                    { "def": "stand-in (1) 1/30", "lane": 1 },
                    { "def": "stand-in (1) 1/30", "lane": 2 },
                    { "def": "stand-in (1) 1/30", "lane": 3 },
                ]),
                "p1",
            );
            let golem = s.unit_at(P1, 2);
            let (left, target, right) = (s.unit_at(P2, 1), s.unit_at(P2, 2), s.unit_at(P2, 3));
            s.attack(&golem, &target);
            assert!(health_of(&s, &left) < 30 && health_of(&s, &right) < 30, "Cleave reached both sides");
            assert!(allured(&s, &target));
            assert!(!allured(&s, &left) && !allured(&s, &right));
        }
    }
}
