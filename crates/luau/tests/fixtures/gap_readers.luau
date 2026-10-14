--!strict
local J = require("@jackioh")

-- Gaps 5, 6, 7, 9 and 12: the turn reader, `given_lucky_on`, the action's
-- events window, `J.config`, `chancePercent`, the self-snapshot `killer_of`,
-- and `nil` (not a truthy null) for readers that answer nothing.
local base = {
	cry = function(ctx: J.EffectContext)
		local me = ctx:controller()
		local turn = ctx:turn()
		local hand = ctx:zone_cards(me, "hand")
		local first = hand[1]
		local missing = ctx:slot_of("no-such-card")
		return {
			J.remember({
				key = "gap579",
				value = {
					turnNumber = turn.number,
					activeIsMe = turn.active == me,
					lucky = if first ~= nil then ctx:given_lucky_on(first) else -1,
					eventsFrom = ctx:events_from(),
					configHealth = J.config.HERO_HEALTH,
					configChain = J.config.CHAIN_OF_THOUGHT_REPEATS,
					missingIsNil = missing == nil,
					missingType = type(missing),
					leftField = ctx:left_field_since_resolved({
						type = "cardPlayed",
						player = me,
						instanceId = "x",
						defId = "lx-fx-001",
						costPaid = 0,
					}),
				},
			}),
			J.radiant_chance({ zone = "hand", chancePercent = 0 }),
		}
	end,
	death = function(ctx: J.EffectContext)
		return {
			J.remember({
				key = "gap9_killer",
				value = { killerIsNil = ctx:killer_of_self() == nil },
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
