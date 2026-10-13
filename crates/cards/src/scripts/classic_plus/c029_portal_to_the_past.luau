--!strict
local J = require("@jackioh")

-- The step the Discover's answer re-enters (§10.6).
local PICKED = "picked"

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.discover_from_last_board({ step = PICKED }),
		}
	end,
	resume = {
		[PICKED] = function(ctx: J.EffectContext)
			return {
				J.add_from_last_board({ costOverride = ctx:param("setCost") }),
			}
		end,
	},
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_random_from_last_board({
				count = ctx:param("cards"),
				costOverride = ctx:param("setCost"),
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
