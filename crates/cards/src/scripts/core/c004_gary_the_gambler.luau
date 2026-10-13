--!strict
local J = require("@jackioh")

local base = {
	cry = function(ctx: J.EffectContext)
		local heads: J.Keyword = { kind = "Divine Shield" }
		local tails: J.Keyword = { kind = "Rush" }
		return {
			J.flip_coins({
				target = { of = "self" },
				coins = ctx:param("coins"),
				perHeads = { attack = ctx:param("perHeads") },
				perTails = { health = ctx:param("perTails") },
			}),
			J.flip_coin_keyword({
				target = { of = "self" },
				headsKeyword = heads,
				tailsKeyword = tails,
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
