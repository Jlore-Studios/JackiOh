--!strict
local J = require("@jackioh")

-- Cast on draw, as M #28.1: heal its caster's hero `heal`, with no cap. Both faces share the one
-- script: only the declared `heal` differs (7, radiant 14).
local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		local target: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = target, amount = ctx:param("heal") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
