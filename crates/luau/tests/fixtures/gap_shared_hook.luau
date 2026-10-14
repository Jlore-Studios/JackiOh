--!strict
local J = require("@jackioh")

-- Gap 10: one function under two hooks. Both faces run the one `tick`, so
-- the loader shares one adapter for both hook keys.
local function tick(_ctx: J.EffectContext)
	return {
		J.gain_mana({ amount = 0 }),
	}
end

local base = {
	startOfTurn = tick,
	endOfTurn = tick,
}

return {
	base = base,
	radiant = "base",
}
