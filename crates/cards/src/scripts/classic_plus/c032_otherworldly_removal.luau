--!strict
local J = require("@jackioh")

-- §8.7 C+ #32: Execute, Brawl, Blade Storm — the order they are added in.
local TOKENS = { "classicplus-032-1", "classicplus-032-2", "classicplus-032-3" }

local base = {
	cry = function(ctx: J.EffectContext)
		local effects = {}
		for _, def_id in ipairs(TOKENS) do
			effects[#effects + 1] = J.add_to_hand({ defId = def_id })
		end
		return effects
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		local effects = {}
		for _, def_id in ipairs(TOKENS) do
			effects[#effects + 1] = J.add_to_hand({ defId = def_id, radiant = true })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
