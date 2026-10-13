--!strict
local J = require("@jackioh")

return {
	-- "After this attacks, draw {draw}."
	base = {
		afterAttack = function(ctx: J.EffectContext)
			return { J.draw({ count = ctx:param("draw") }) }
		end,
	},
	-- "After this attacks, Recruit {recruits|card|cards}.": one top-down scan per recruit.
	radiant = {
		afterAttack = function(ctx: J.EffectContext)
			local effects = {}
			for _ = 1, ctx:param("recruits") do
				effects[#effects + 1] = J.recruit({ player = "self" })
			end
			return effects
		end,
	},
}
