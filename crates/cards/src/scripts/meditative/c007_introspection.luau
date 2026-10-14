--!strict
local J = require("@jackioh")

-- End of turn, in the written order: heal the hero, then each of your Units (C+ #19.4 Support
-- Loser's list); draw; discount the whole hand, the drawn card included; then one hit on each enemy,
-- every hit landing before the state check (R59). The heal and the discount reach their `each`
-- hooks through `data`. One table serves both faces: every number but the draw is declared per face
-- and read through `param` (R386).
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		local heal = ctx:param("heal")
		local draws = ctx:param("draw")
		local discount = ctx:param("discount")
		local damage = ctx:param("damage")
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = hero, amount = heal }),
			J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = heal } }),
			J.draw({ count = draws }),
			J.for_each_card({ cards = "each_cards_2", each = "each_card_2", data = { amount = -discount } }),
			J.damage_all({ amount = damage, side = "enemy", heroes = true }),
		}
	end,
	-- Each of your Units, as the heal reaches them.
	each_cards = function(ctx: J.EffectContext): { string }
		local ids: { string } = {}
		for _, card in ipairs(ctx:cards_in_scope({ side = "self" })) do
			ids[#ids + 1] = card.id
		end
		return ids
	end,
	each_card = function(id: string, data: any)
		local unit: J.TargetSpec = { of = "instance", instanceId = id }
		return { J.heal({ target = unit, amount = data.amount }) }
	end,
	-- The hand after the draw.
	each_cards_2 = function(ctx: J.EffectContext): { string }
		local ids: { string } = {}
		for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
			ids[#ids + 1] = card.id
		end
		return ids
	end,
	each_card_2 = function(id: string, data: any)
		return { J.set_cost_mod({ target = { of = "instance", instanceId = id }, amount = data.amount }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
