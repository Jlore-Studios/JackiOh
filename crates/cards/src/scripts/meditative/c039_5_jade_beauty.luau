--!strict
local J = require("@jackioh")

-- Each end of its controller's turn (R62): ME-ALLURE over every enemy Unit then on the field
-- (MD-C4). Both faces Allure alike; the Radiant face's stats and keywords are catalog data.
local base = {
	endOfTurn = function(_ctx: J.EffectContext)
		return { J.allure_enemy_units() }
	end,
}

return {
	base = base,
	radiant = "base",
}
