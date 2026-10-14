--!strict
local J = require("@jackioh")

-- §8.7: "If it costs (1) or less", Radiant "(2) or less". An AI card declares no params (B8).
local MAX_COST_BASE = 1
local MAX_COST_RADIANT = 2

-- The chain is one engine verb (`draw_while_cheap`): at most `CHAIN_OF_THOUGHT_REPEATS` repeats
-- after the first draw.
local function chain(max_cost: number)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.draw_while_cheap({ maxCost = max_cost, repeats = J.config.CHAIN_OF_THOUGHT_REPEATS }),
			}
		end,
	}
end

return {
	base = chain(MAX_COST_BASE),
	radiant = chain(MAX_COST_RADIANT),
}
