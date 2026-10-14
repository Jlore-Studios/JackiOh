--!strict
local J = require("@jackioh")

-- Aura: its controller's hero takes at most {cap} from each damage instance (§4.4 step 3).
local base = {
	heroGuard = function(ctx: J.HookArgs)
		local guard: J.HeroGuard = { cap = ctx:param("cap") }
		return { guard }
	end,
}

-- The Radiant face adds Reborn and doubles the stats: catalog data only.
return {
	base = base,
	radiant = "base",
}
