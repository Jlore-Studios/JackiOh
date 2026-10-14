--!strict
local J = require("@jackioh")

-- "Lose all mana next turn": one lost refresh (ME-TURN, R844).
local LOST_TURNS = 1
-- The Radiant face's Echo 1.
local ECHO = 1

-- A declared hand pick (R81, as C+ #41's): another of your hand cards that is not Radiant yet.
local TARGETS = {
	{
		kind = "hand",
		min = 1,
		max = 1,
		filter = { of = { "hand" }, excludeSelf = true, check = "notRadiant" },
	},
}

local TARGET_CHECKS = {
	notRadiant = function(args)
		local candidate = args:candidate()
		return candidate ~= nil and not candidate.radiant
	end,
}

-- The lost refresh, then the chosen card made Radiant (§5.2); with no pick it still resolves.
local function cry(ctx: J.EffectContext)
	return {
		J.lose_refreshes({ turns = LOST_TURNS }),
		J.set_radiant({ target = { of = "chosen" } }),
	}
end

return {
	base = {
		staticFlags = { quickdraw = true },
		targets = TARGETS,
		targetChecks = TARGET_CHECKS,
		cry = cry,
	},
	-- The Echo repeat reopens the pick as a `hand` prompt (§10.6).
	radiant = {
		staticFlags = { quickdraw = true, echo = ECHO },
		targets = TARGETS,
		targetChecks = TARGET_CHECKS,
		cry = cry,
	},
}
