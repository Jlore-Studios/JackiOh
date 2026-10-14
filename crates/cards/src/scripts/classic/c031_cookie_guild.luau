--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		local costLimit = ctx:param("costLimit")
		local effects = {}
		for _ = 1, ctx:param("units") do
			effects[#effects + 1] = J.recruit({
				filter = { type = "Unit", costRange = { max = costLimit } },
				player = "self",
			})
		end
		return effects
	end,
}

-- The same script: the Radiant face's three scans are its declared `units`, which `param` reads off the running face.
return {
	base = base,
	radiant = "base",
}
