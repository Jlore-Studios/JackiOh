--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy_all({ side = "self", rows = { "backrow" } }),
		}
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy_all({ side = "any", rows = { "backrow" } }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
