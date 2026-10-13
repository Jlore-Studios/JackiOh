--!strict
local J = require("@jackioh")

local function solarius_prime(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.cast_random({
					query = { type = "Spell" },
					count = ctx:param("casts"),
					radiant = radiant,
					targetEnemies = true,
				}),
			}
		end,
	}
end

return {
	base = solarius_prime(false),
	radiant = solarius_prime(true),
}
