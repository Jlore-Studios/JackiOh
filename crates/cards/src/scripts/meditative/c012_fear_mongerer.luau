--!strict
local J = require("@jackioh")

-- `trigger_turn_hooks` queues its controller's end-of-turn hooks as §2.2's end-of-turn trigger step
-- would (R153, R155, R821, R824). The same script on both faces: the base face reads its printed
-- `times` (1, R749), the Radiant face its 2.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.trigger_turn_hooks({ times = ctx:param("times") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
