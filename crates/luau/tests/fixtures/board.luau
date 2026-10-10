-- Two readers and two effects, the same on both faces.
local J = require("@jackioh")

local base = {
	cry = function(ctx)
		local hero = ctx:hero_of(ctx:controller())
		return {
			J.damage({ to = { of = "enemyHero" }, amount = J.div(hero.health, 7) }),
			J.destroy({ target = { of = "chosen" } }),
		}
	end,
}

return {
	base = base,
	radiant = base,
}
