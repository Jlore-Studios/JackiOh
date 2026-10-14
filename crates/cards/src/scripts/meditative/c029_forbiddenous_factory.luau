--!strict
local J = require("@jackioh")

-- M #28.1 Ancient Curse, the token the end-of-turn clause shuffles (§7).
local CURSE = "meditative-028-1"

-- "Spend a Plague Counter": one a turn, the price of the shuffle.
local SPENT = 1

-- Both faces share the one script: the declared `counters` (3 and 3) and `curses` (1, radiant 2)
-- are what differ.
local base = {
	-- Played or cast (R1): one placement of `counters` on itself (§6.3, R689), R471's multiplier
	-- applying.
	cry = function(ctx: J.EffectContext)
		return { J.place_plague({ amount = ctx:param("counters") }) }
	end,
	-- With no counter left it does nothing: "Spend … to …" is a price.
	endOfTurn = function(ctx: J.EffectContext)
		local me = ctx:live_self()
		if me == nil or ctx:plague_on(me.id) < SPENT then
			return {}
		end
		return {
			J.consume_plague({ amount = SPENT }),
			J.shuffle_into({ defId = CURSE, count = ctx:param("curses"), player = "enemy" }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
