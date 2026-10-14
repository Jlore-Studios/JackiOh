--!strict
local J = require("@jackioh")

-- Reads a spread of board facts into one remembered table so tests can run
-- the effect and read every value back out of the context's data bag.
local base = {
	cry = function(ctx: J.EffectContext)
		local me = ctx:controller()
		local foe = ctx:opponent_of(me)
		return {
			J.remember({
				key = "probe",
				value = {
					controller = me,
					radiant = ctx:radiant(),
					x = ctx:x(),
					fib = ctx:fib(10),
					myHeroHealth = ctx:hero_of(me).health,
					foeHeroHealth = ctx:hero_of(foe).health,
					myHand = #ctx:zone_cards(me, "hand"),
					myMana = ctx:unspent_mana_of(me),
					myMaxMana = ctx:max_mana_of(me),
					myUnits = #ctx:active_units_of(me),
					foeUnits = #ctx:active_units_of(foe),
					played = ctx:cards_played_this_turn(me),
					graveyard = #ctx:zone_cards(me, "graveyard"),
					missingTags = #ctx:tags_of("no-such-card"),
					missingCost = ctx:printed_cost("no-such-card"),
					summoned = #ctx:summoned_so_far(),
					chosen = #ctx:chosen_options(),
				},
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
