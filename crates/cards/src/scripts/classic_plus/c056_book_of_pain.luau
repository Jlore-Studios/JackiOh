--!strict
local J = require("@jackioh")

-- The same script on both faces: the Radiant face's 4 is its declared `discards`, which `param` reads.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.discard_random({ count = ctx:param("discards"), player = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
