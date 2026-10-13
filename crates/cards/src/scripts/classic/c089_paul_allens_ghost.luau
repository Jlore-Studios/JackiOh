--!strict
local J = require("@jackioh")

-- B5 E5: to target this with anything but an attack, a player must also discard the declared
-- `discard` cards, random at pay time (R682).
local base = {
	targetingDiscards = function(ctx: J.HookArgs)
		return ctx:param("discard")
	end,
}

-- The same script: the Radiant face differs only in what the engine reads off the catalog (its doubled
-- stats and Reborn).
return {
	base = base,
	radiant = "base",
}
