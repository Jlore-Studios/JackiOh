--!strict
local J = require("@jackioh")

-- "All your units become Radiant", as one `setRadiant` per unit named by instance id: nobody chose
-- these cards, so there is no `TargetSpec` to resolve (R81). R78: by the time Death runs she is in
-- the graveyard, so "your OTHER units" is simply everyone left standing.
local function radiate_your_units(ctx: J.EffectContext): { J.EffectCall }
	local effects = {}
	for _, unit in ipairs(ctx:active_units_of(ctx:controller())) do
		effects[#effects + 1] = J.set_radiant({ instanceId = unit.id })
	end
	return effects
end

local base = {
	death = function(ctx: J.EffectContext)
		return radiate_your_units(ctx)
	end,
}

-- Radiant Death: the board first, in the order the text names it, then the hand, card by card, as
-- #29 does. Every card, so no rng is drawn (§9.3), and a card already Radiant is still named:
-- `setRadiant` cues it without changing it (R177), which hides the hand's faces from the opponent.
local radiant = {
	death = function(ctx: J.EffectContext)
		local effects = radiate_your_units(ctx)
		for _, card in ipairs(ctx:zone_cards(ctx:controller(), "hand")) do
			effects[#effects + 1] = J.set_radiant({ instanceId = card.id })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
