--!strict
local J = require("@jackioh")

-- "Deal {damage} damage to each enemy": one hit on every enemy unit and the enemy hero, all landing
-- before the state check (R59).
local function storm(ctx: J.EffectContext)
	return {
		J.damage_all({
			amount = ctx:param("damage"),
			side = "enemy",
			heroes = true,
		}),
	}
end

local base = {
	staticFlags = { castOnDraw = true },
	cry = storm,
}

-- "Echo 1" plus the same: the `echo` flag resolves it a second time (R51's repeat).
local radiant = {
	staticFlags = { castOnDraw = true, echo = 1 },
	cry = storm,
}

return {
	base = base,
	radiant = radiant,
}
