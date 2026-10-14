--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.transform_beneath({}),
		}
	end,
}

-- The Radiant face adds First Strike and doubles the stats (catalog data); its copies are Radiant as it is.
return {
	base = base,
	radiant = "base",
}
