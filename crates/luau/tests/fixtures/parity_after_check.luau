--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.after_state_check({ hook = "after_check", data = { amount = 2 } }),
		}
	end,
	after_check = function(ctx: J.EffectContext, data: J.JsonValue)
		return {
			J.damage({ to = { of = "enemyHero" }, amount = data.amount }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
