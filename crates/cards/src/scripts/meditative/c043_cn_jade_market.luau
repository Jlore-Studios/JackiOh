--!strict
local J = require("@jackioh")

-- The Rock this adds (M #39.1).
local ROCK = "meditative-039-1"

local function market(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			-- R348: X is at least 1 (the play validator refused 0 and above-mana); `ctx:x()` is the
			-- X paid as Nerf and Buff tuned it (§6.3's X row), and `max(0)` keeps a tuned-down X
			-- from underflowing.
			local x = math.max(ctx:x(), 0)
			local effects = {}
			for _ = 1, x do
				effects[#effects + 1] = J.add_to_hand({ defId = ROCK, radiant = radiant })
			end
			return effects
		end,
	}
end

return {
	base = market(false),
	radiant = market(true),
}
