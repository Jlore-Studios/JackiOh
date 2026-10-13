--!strict
local J = require("@jackioh")

-- §8.7: "(1) more", Radiant "(2) more". An AI card declares no params (B8), so its numbers live here.
local TAX = { base = 1, radiant = 2 }

local function tax(amount: number)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.add_cost_rule({
					player = "enemy",
					rule = { amount = amount },
					lasts = "theirNextTurn",
				}),
			}
		end,
	}
end

return {
	base = tax(TAX.base),
	radiant = tax(TAX.radiant),
}
