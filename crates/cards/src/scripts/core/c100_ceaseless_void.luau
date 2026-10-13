--!strict
local J = require("@jackioh")

local ID = "core-100"

-- R55: "Costs 1 less per card drawn, played, destroyed or exiled this game by either player",
-- floored at 0 (§8.5's Engine cell). The printed 100 is R65's reading of the catalog (`J.cost`),
-- never restated; the four counters are game-level, so neither side's identity is read and nothing
-- is stored on the instance. The 1 is the declared number `discount` (R386).
local function cost(ctx: J.CostArgs): number
	local discount = ctx:param("discount")
	local counters = ctx:counters()
	local spent = counters.drawn + counters.played + counters.destroyed + counters.exiled
	return math.max(J.cost(ID) - discount * spent, 0)
end

-- One script for both faces: "Plus Charge" is a printed keyword on the catalog's radiant face and
-- the 20/20 its printed stats, so both are §10.4 layer 1 and not a line of script.
local base = {
	cost = cost,
	-- "Cry: exile all other permanents on both sides": §3.1's two rows of both sides (§5.1's
	-- permanents), this unit excluded (R68, R59, R13, R11).
	cry = function(ctx: J.EffectContext)
		return {
			J.exile_all({ side = "any", rows = { "units", "backrow" }, excludeSelf = true }),
		}
	end,
	-- R662: lit in hand once `play_cost` (R65, every discount) is within its controller's mana,
	-- whether or not a zone is free (the cost itself is on the face, R280).
	conditionMet = function(ctx: J.ConditionContext)
		return ctx:zone() == "hand" and ctx:play_cost() <= ctx:unspent_mana_of(ctx:controller())
	end,
}

return {
	base = base,
	radiant = "base",
}
