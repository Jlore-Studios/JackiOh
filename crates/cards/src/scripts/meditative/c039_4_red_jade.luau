--!strict
local J = require("@jackioh")

-- ME-JADE's `add_jade`. The same script: the Radiant face's 10 is its declared `jade`, which
-- `param` reads off the running face.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_jade({ amount = ctx:param("jade") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
