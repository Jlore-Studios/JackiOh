--!strict
local J = require("@jackioh")

-- The engine's Cry and Death multiplier (`cryDeathExtra`, R820, R822, R823): while this acts on
-- its controller's field, their Cries and Deaths run `extra` more times. Both faces run this one
-- script; `param` reads the running face's `extra` (1, Radiant 2).
local base = {
	cryDeathExtra = function(ctx: J.HookArgs): number
		return ctx:param("extra")
	end,
}

return {
	base = base,
	radiant = "base",
}
