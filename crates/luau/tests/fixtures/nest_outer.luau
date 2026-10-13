-- A hook that calls a reader which runs another card's hook (L8's nesting).
local J = require("@jackioh")

return {
	base = {
		cry = function(ctx)
			local kinds = ctx:effects_of("fixture-nest-inner", "base", "cry")
			return { J.damage({ to = { of = "enemyHero" }, amount = #kinds }) }
		end,
	},
}
