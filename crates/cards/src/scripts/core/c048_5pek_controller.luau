--!strict
local J = require("@jackioh")

-- §8.2's two radiant options, as the play action spells them (R81).
local ENEMY = "enemy"
local ALL = "all"

local base = {
	cry = function(ctx: J.EffectContext)
		return { J.switch_all_positions({ side = "both" }) }
	end,
}

local radiant = {
	modes = {
		{ kind = "mode", options = { ENEMY, ALL } },
	},
	cry = function(ctx: J.EffectContext)
		-- R81: the mode arrived with the play, and R90 validated it against this declaration, so the
		-- only way it is anything else is that no mode was carried. "All units" is what the card does
		-- when it is not narrowed, so that is the reading an unnarrowed play gets.
		local mode = ctx:chosen_options()[1]
		local side: J.SwitchAllPositionsSide = "both"
		if mode == ENEMY then
			side = "enemy"
		end
		return { J.switch_all_positions({ side = side }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
