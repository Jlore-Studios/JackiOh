--!strict
local J = require("@jackioh")

-- The replacement both faces share; the radiant face adds the Cry on top of it ("same").
local base = {
	staticFlags = { infiniteReserves = true },
}

local radiant = {
	staticFlags = { infiniteReserves = true },
	-- "Cry: Draw 3": the declared number `draw` (R386).
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
