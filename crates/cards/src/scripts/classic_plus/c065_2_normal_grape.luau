--!strict
local J = require("@jackioh")

-- The same script on both faces: the Radiant face's 4 and 2 are its declared `amount` and `draw`,
-- which `param` reads.
local base = {
	-- §8.7: "an enemy or an ally" — any Unit or hero, either side.
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "hero" } },
		},
	},
	cry = function(ctx: J.EffectContext)
		local effects = {
			J.damage_enemy_or_heal_friend({ amount = ctx:param("amount") }),
		}
		-- §8.7: "It costs (1) less" — a discount that stacks with every other modifier (R65), the
		-- declared number `discount` (R386). "Draw N" is N separate draws (§2.4).
		local discount = -ctx:param("discount")
		for _ = 1, ctx:param("draw") do
			effects[#effects + 1] = J.draw_priced({ costMod = discount })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = "base",
}
