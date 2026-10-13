--!strict
local J = require("@jackioh")

-- §6.3 "Tribute 3": three of your Units, or one Sheep Token plus one more, worth 3 (§3.2).
local TRIBUTE_COST = 3

-- One script serves both faces: Tribute 3 is its static flag, and Immutable (R23) and the Radiant
-- face's Armor 2 and 0/100 are catalog data.
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
	-- R1260: all five of the Wall's side, its own zone and the occupied ones included; `lock` skips
	-- a zone Locked already and evicts no one.
	cry = function(ctx: J.EffectContext)
		local effects = {}
		for lane = 1, J.config.UNIT_ZONES do
			effects[#effects + 1] = J.lock({ zone = { of = "lane", row = "units", lane = lane } })
		end
		return effects
	end,
}

return {
	base = base,
	radiant = "base",
}
