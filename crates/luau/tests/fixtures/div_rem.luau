-- `J.div` and `J.rem` over negative numbers, and a division by zero.
local J = require("@jackioh")

local function hit(amount)
	return J.damage({ to = { of = "enemyHero" }, amount = amount })
end

return {
	base = {
		cry = function(ctx)
			return {
				hit(J.div(-7, 2)),
				hit(J.rem(-7, 2)),
				hit(J.div(7, -2)),
				hit(J.rem(7, -2)),
				hit(J.div(-7, -2)),
				hit(J.rem(-7, -2)),
			}
		end,
		death = function(ctx)
			return { hit(J.div(1, 0)) }
		end,
	},
}
