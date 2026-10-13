--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.place_plague_tokens({ count = ctx:param("tokens") }),
		}
	end,
}

-- The same script: the Radiant face's 10 is its declared `tokens`.
return {
	base = base,
	radiant = "base",
}
