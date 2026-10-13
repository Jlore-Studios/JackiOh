--!strict
local J = require("@jackioh")

-- End of turn: heal your hero, then each of your Units, {heal}. The amount is read once, as the
-- hook runs, and carried to each heal in `data`. The same script on both faces: Reborn and the
-- Radiant heal 6 are catalog data.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		local amount = ctx:param("heal")
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = hero, amount = amount }),
			J.for_each_card({ cards = "each_cards", each = "each_card", data = { amount = amount } }),
		}
	end,
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
}

return {
	base = base,
	radiant = "base",
}
