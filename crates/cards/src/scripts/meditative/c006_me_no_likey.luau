--!strict
local J = require("@jackioh")

-- "Give one of your Units to your opponent." The gift moves by `give_control` (R1423), then pays
-- its cost on the field (R803, R65, R396) only if it now stands on the opponent's side: the
-- `for_each_card` reads that as the list reaches it, after the move. The Radiant face differs only
-- in its declared `multiple` (2), read through `param`.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "ally",
				of = { "unit" },
			},
		},
	},
	cry = function(ctx: J.EffectContext)
		local unit = ctx:instance_of({ of = "chosen" })
		if unit == nil then
			return {}
		end
		-- The gift's price, read before it moves (R803, R65, R396).
		local gain = ctx:cost_now(unit.id) * ctx:param("multiple")
		return {
			J.give_control({ instanceId = unit.id }),
			J.for_each_card({
				cards = "each_cards",
				each = "each_card",
				data = { moved = unit.id, gain = gain },
			}),
		}
	end,
	-- "If you do": only a Unit that now stands on the opponent's side pays out.
	each_cards = function(ctx: J.EffectContext, data: any): { string }
		local enemy = ctx:opponent_of(ctx:controller())
		local changed_sides = false
		for _, card in ipairs(ctx:active_units_of(enemy)) do
			if card.id == data.moved then
				changed_sides = true
			end
		end
		if data.gain > 0 and changed_sides then
			return { data.moved }
		end
		return {}
	end,
	each_card = function(_id: string, data: any)
		return { J.gain_mana({ amount = data.gain }) }
	end,
}

return {
	base = base,
	radiant = "base",
}
