--!strict
local J = require("@jackioh")

-- End of turn: one hit on the enemy hero, then the card's own `damage` set to the damage plus the
-- growth (R386's `tuning.set`, R825), even when Armor took the hit whole. Both faces run one script;
-- `param` reads the face's numbers.
local base = {
	endOfTurn = function(ctx: J.EffectContext)
		local hit = ctx:param("damage")
		local growth = ctx:param("growth")
		local this: J.TargetSpec = { of = "self" }
		return {
			J.damage({ to = { of = "enemyHero" }, amount = hit }),
			J.set_number({ target = this, which = "param:damage", value = hit + growth }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
