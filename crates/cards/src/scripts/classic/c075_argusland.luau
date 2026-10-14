--!strict
local J = require("@jackioh")

-- The engine's hero guard (`Script.heroGuard`): a hero damage divisor after Armor and before the hit
-- caps; several multiply.
local base = {
	heroGuard = function(ctx: J.HookArgs)
		local guard: J.HeroGuard = { divisor = ctx:param("divisor") }
		return { guard }
	end,
}

-- The same script: the Radiant face's 4 is its declared `divisor`, which `param` reads off the running face.
return {
	base = base,
	radiant = "base",
}
