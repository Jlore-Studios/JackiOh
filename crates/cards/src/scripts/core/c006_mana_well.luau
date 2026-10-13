--!strict
local J = require("@jackioh")

-- The only difference between the two faces is how much mana the start of the turn gives: the
-- declared number `mana` (R386), 1 and 2 on the Radiant face, read off the face that is up.
local base = {
	startOfTurn = function(ctx: J.EffectContext)
		return {
			J.gain_mana({ amount = ctx:param("mana") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
