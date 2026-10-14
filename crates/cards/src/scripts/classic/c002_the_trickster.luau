--!strict
local J = require("@jackioh")

-- "Trap or Field Spell": a Field Trap is a Trap.
local REACHES: { J.CardType } = { "Trap", "Field Trap", "Field Spell" }

local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.add_cost_rule({
				rule = { types = REACHES, amount = -ctx:param("discount") },
				lasts = "used",
			}),
		}
	end,
}

local radiant = {
	-- "costs (0)": the declared number `setCost` (R386).
	cry = function(ctx: J.EffectContext)
		return {
			J.add_cost_rule({
				rule = { types = REACHES, setTo = ctx:param("setCost") },
				lasts = "used",
			}),
		}
	end,
}

return {
	base = base,
	radiant = radiant,
}
