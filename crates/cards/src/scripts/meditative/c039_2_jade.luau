--!strict
local J = require("@jackioh")

-- ME-JADE's `add_jade`, whose crossings of 5 and 10 run inside the same effect (MD-C3), then
-- `gain_mana`. The same script: the Radiant face's 2s are its declared `jade` and `mana`, which
-- `param` reads off the running face.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_jade({ amount = ctx:param("jade") }),
			J.gain_mana({ amount = ctx:param("mana") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
