--!strict
local J = require("@jackioh")

-- The cap is read by the pipeline (`damage::hero_damage_cap`) as the card's declared number `cap`
-- (R386), 5 and 3 on the Radiant face, and the Cry's "draw 1" (radiant "draw 2") is the declared
-- number `draw`: both faces run this one script.
local base = {
	staticFlags = { antiOneshot = true },
	cry = function(ctx: J.EffectContext)
		return {
			J.draw({ count = ctx:param("draw") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
