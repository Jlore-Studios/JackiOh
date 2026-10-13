--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return { J.swap_library() }
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.swap_library(),
			J.recruit({ count = ctx:param("recruits") }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
