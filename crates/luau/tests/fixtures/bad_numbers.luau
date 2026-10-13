-- Hooks that hand the host numbers that are not i32s (L6), one per hook.
local J = require("@jackioh")

local function hit(amount)
	return { J.damage({ to = { of = "enemyHero" }, amount = amount }) }
end

return {
	base = {
		cry = function(ctx)
			return hit(1.5)
		end,
		death = function(ctx)
			return hit(0 // 0)
		end,
		startOfTurn = function(ctx)
			return hit(1 // 0)
		end,
		endOfTurn = function(ctx)
			return hit(J.div(3.5, 1))
		end,
	},
	radiant = {
		cry = function(ctx)
			return hit(2147483648)
		end,
	},
}
