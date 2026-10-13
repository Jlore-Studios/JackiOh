--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.summon({ defId = "core-t-rush", player = "self" }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		local out = {}
		for _ = 1, ctx:param("tokens") do
			out[#out + 1] = J.summon({ defId = "core-t-rush", player = "self" })
		end
		return out
	end,
}

return {
	base = base,
	radiant = radiant,
}
