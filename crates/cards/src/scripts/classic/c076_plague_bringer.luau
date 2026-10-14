--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.place_plague_tokens({ count = ctx:param("tokens") }),
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

-- The same script: the Radiant face's 4 tokens and draw 2 are its declared numbers.
return {
	base = base,
	radiant = "base",
}
