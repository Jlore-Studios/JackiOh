--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

-- The same script: the Radiant face's 6 is its declared `draw`, which `param` reads off the running face.
return {
	base = base,
	radiant = "base",
}
