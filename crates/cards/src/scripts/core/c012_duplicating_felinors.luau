--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.summon_copy({ of = { of = "self" }, player = "self" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
