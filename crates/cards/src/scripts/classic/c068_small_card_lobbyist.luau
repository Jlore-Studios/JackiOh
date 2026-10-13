--!strict
local J = require("@jackioh")

return {
	-- A threshold rung on every player's cards: `minCost` the declared threshold and `amount` the
	-- declared surcharge (R363).
	base = {
		costAura = function(ctx: J.HookArgs)
			local rule: J.CostAura = {
				minCost = ctx:param("threshold"),
				amount = ctx:param("surcharge"),
				whose = "all",
			}
			return { rule }
		end,
	},
	-- A ban on the opponent's cards whose finished price is the threshold or more (R65).
	radiant = {
		costAura = function(ctx: J.HookArgs)
			local rule: J.CostAura = {
				minCost = ctx:param("threshold"),
				whose = "opponents",
				ban = true,
			}
			return { rule }
		end,
	},
}
