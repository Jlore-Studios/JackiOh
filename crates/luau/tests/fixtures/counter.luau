-- A module-level counter a hook increments: the chunk runs afresh on every call, so it reads 1 every
-- time (L8).
local J = require("@jackioh")

local calls = 0

return {
	base = {
		cry = function(ctx)
			calls += 1
			return { J.damage({ to = { of = "enemyHero" }, amount = calls }) }
		end,
	},
}
