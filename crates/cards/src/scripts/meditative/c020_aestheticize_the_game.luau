--!strict
local J = require("@jackioh")

local HEALTH_MODE = "health"
local GRAVEYARD_MODE = "graveyard"
local BOARD_MODE = "board"

-- ME-WIN's chosen condition (R848–R850): the declared mode names the condition, and its threshold is
-- the declared number of the same name (R386). The Radiant face's draw is R275's Spell rider.
local base = {
	staticFlags = { quickdraw = true },
	modes = {
		{ kind = "mode", options = { HEALTH_MODE, GRAVEYARD_MODE, BOARD_MODE } },
	},
	cry = function(ctx: J.EffectContext)
		local mode = ctx:chosen_options()[1]
		if mode == nil then
			return {}
		end
		local threshold = ctx:param(mode)
		local effects: { J.EffectCall } = {
			J.alt_win({ condition = mode :: any, threshold = threshold }),
		}
		if ctx:radiant() then
			effects[#effects + 1] = J.draw({ count = ctx:param("draw") })
		end
		return effects
	end,
}

-- The Radiant face's 90s and draw are its declared numbers; the modes are both faces'.
return {
	base = base,
	radiant = "base",
}
