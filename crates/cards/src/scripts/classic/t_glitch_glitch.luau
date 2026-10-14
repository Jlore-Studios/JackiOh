--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return { J.glitch() }
	end,
}

return {
	base = base,
	radiant = "base",
}
