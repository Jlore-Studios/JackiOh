--!strict
local J = require("@jackioh")

-- R280: the part of the text the value belongs to, on both faces.
local X_LABEL = "X = cards you played earlier this turn"

-- R281: the same grant and the same X on both faces; the Radiant face's 2X is the multiple the engine
-- reads off the instance's face (`QUICKSTRIKER_COMBO_MULTIPLE`), as it reads #84's Armor.
local base = {
	staticFlags = { quickstriker = true },
	preview = function(ctx: J.ConditionContext)
		return {
			{
				label = X_LABEL,
				value = ctx:cards_played_this_turn(ctx:controller()),
			},
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
