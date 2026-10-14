--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "selfHero" }
		return {
			J.damage({ to = target, amount = 5 }),
			J.heal({ target = target, amount = 3 }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
