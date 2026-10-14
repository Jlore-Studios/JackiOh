--!strict
local J = require("@jackioh")

-- A misspelled sub-hook name: the face holds `after_check`, the verb names
-- `after_chek`. The verb fails when it builds, naming the card, face and name.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.after_state_check({ hook = "after_chek" }),
		}
	end,
	after_check = function(ctx: J.EffectContext, data: J.JsonValue)
		return {}
	end,
}

return {
	base = base,
	radiant = "base",
}
