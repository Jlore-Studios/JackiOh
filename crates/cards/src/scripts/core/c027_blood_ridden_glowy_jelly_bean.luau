--!strict
local J = require("@jackioh")

-- §8.2: the blood price is the same on both faces; only the number of cards changes. Both are
-- declared numbers (R386): `loss`, 5, less being better, and `cards`, 2 on the Radiant face and tuned
-- there alone (R749: the base face's "a random card" prints no number and is always 1).
local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		return {
			J.set_radiant_random({ zones = "hand", count = ctx:param("cards") }),
			J.lose_health({ player = "self", amount = ctx:param("loss") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
