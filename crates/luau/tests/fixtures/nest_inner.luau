-- The card another fixture's hook runs from a reader.
local J = require("@jackioh")

return {
	base = {
		cry = function(ctx)
			return {
				J.damage({ to = { of = "enemyHero" }, amount = 2 }),
				J.destroy({ target = { of = "self" } }),
			}
		end,
	},
}
