--!strict
local J = require("@jackioh")

-- B5 E19, R471: what each placement onto this card is multiplied by, the declared `multiplier`.
local base = {
	plagueMultiplier = function(ctx: J.HookArgs)
		return ctx:param("multiplier")
	end,
}

-- The same script: the Radiant face's ×3 is its declared `multiplier`, which `param` reads off the face it wears.
return {
	base = base,
	radiant = "base",
}
