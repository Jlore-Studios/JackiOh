--!strict
local J = require("@jackioh")

-- "Refresh your mana": every spent crystal, capped at max mana (R364), whatever the player's max is
-- (§9.9's handicaps). `i32::MAX` halved: the Refresh adds this to current mana before capping, so
-- the sum stays in range while still more than any max mana.
local ALL_MANA = 1073741823

-- The faces differ only in which face the scorer ranks and the new cards arrive with.
local function zealotism(radiant: boolean): { J.EffectCall }
	return {
		J.replace_hand_with_perfect({ radiant = radiant }),
		J.refresh_mana({ amount = ALL_MANA }),
	}
end

return {
	base = {
		cry = function(ctx: J.EffectContext)
			return zealotism(false)
		end,
	},
	radiant = {
		cry = function(ctx: J.EffectContext)
			return zealotism(true)
		end,
	},
}
