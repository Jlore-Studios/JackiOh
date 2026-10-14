--!strict
local J = require("@jackioh")

-- The same script on both faces: the Radiant face's 10 and 2 are its declared `amount` and `draw`,
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
		-- §8.7: "Each costs (0)": the declared number `setCost` (R386), less being better.
		local cost = ctx:param("setCost")
		for _ = 1, ctx:param("draw") do
			effects[#effects + 1] = J.draw_priced({ costOverride = cost })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = "base",
}
