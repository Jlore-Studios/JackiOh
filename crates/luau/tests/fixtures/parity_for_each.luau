--!strict
local J = require("@jackioh")

-- One damage to every enemy unit, twice, dealt through two `for_each_card`s in one
-- hook with different named functions: a second use appends `_2`. The lists come
-- from a reader, the blows from a function of the id.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({
				cards = "each_cards",
				each = "each_card",
			}),
			J.for_each_card({
				cards = "each_cards_2",
				each = "each_card_2",
			}),
		}
	end,
	each_cards = function(ctx: J.EffectContext)
		local ids = {}
		for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			ids[#ids + 1] = unit.id
		end
		return ids
	end,
	each_card = function(id: string)
		return { J.damage({ to = { of = "instance", instanceId = id }, amount = 1 }) }
	end,
	each_cards_2 = function(ctx: J.EffectContext)
		local ids = {}
		for _, unit in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			ids[#ids + 1] = unit.id
		end
		return ids
	end,
	each_card_2 = function(id: string)
		return { J.damage({ to = { of = "instance", instanceId = id }, amount = 1 }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
