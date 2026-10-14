--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		local out = {}
		for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
			out[#out + 1] = J.set_radiant({ instanceId = card.id })
		end
		return out
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		local me = ctx:controller()
		local out = {}
		for _, card in ipairs(ctx:zone_cards(me, "hand")) do
			out[#out + 1] = J.set_radiant({ instanceId = card.id })
		end
		for _, unit in ipairs(ctx:active_units_of(me)) do
			out[#out + 1] = J.set_radiant({ instanceId = unit.id })
		end
		for _, card in ipairs(ctx:permanents_on_field(me)) do
			if card.zone.player == me and card.zone.row == "backrow" then
				out[#out + 1] = J.set_radiant({ instanceId = card.id })
			end
		end
		return out
	end,
}

return {
	base = base,
	radiant = radiant,
}
