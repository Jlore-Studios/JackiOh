--!strict
local J = require("@jackioh")

local base = {
	-- The enemy's units, read as the Cry runs: the list's first and only effect reads its set at this
	-- moment, so it is the set the Cry resolves against (R113); none leaves `cards` out, for an empty
	-- table crosses as a record, not a list.
	cry = function(ctx: J.EffectContext)
		local ids: { string } = {}
		for _, card in ipairs(ctx:active_units_of(ctx:opponent_of(ctx:controller()))) do
			ids[#ids + 1] = card.id
		end
		return {
			J.for_each_card({
				cards = if #ids > 0 then ids else nil,
				each = "each_card",
			}),
		}
	end,
	each_card = function(id: string)
		return {
			J.switch_position_of({
				to = "DEF",
				target = { of = "instance", instanceId = id },
			}),
		}
	end,
}

-- The same script: the Radiant face's 4/8 and its added Rush are catalog data.
return {
	base = base,
	radiant = "base",
}
