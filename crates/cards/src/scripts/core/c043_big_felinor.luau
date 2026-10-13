--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy_all({ side = "any", notTags = { "Felinor" } }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy_all({ side = "enemy", notTags = { "Felinor" } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
