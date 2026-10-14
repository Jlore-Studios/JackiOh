--!strict
local J = require("@jackioh")

-- §8.8: "Activate" is once a turn, the Radiant face's "Activate 2" twice.
local BASE_USES = 1
local RADIANT_USES = 2

-- §6.3 "Tribute 5": five of your Units, or Sheep Tokens worth as many (§3.2).
local TRIBUTE_COST = 5

-- C+ #2 Groom Shroom's fill: each empty, unlocked, unreserved unit zone, left to right (R64), gets a
-- random non-token Jlockeed Unit of every set (R380; this token is out, §5.1, R387), repeats allowed
-- (R60), summoned, so with no Cry (R1).
local function fill(uses: number, radiant: boolean)
	return {
		id = "fill",
		label = "Fill your board",
		uses = uses,
		run = function(ctx: J.EffectContext)
			local effects: { J.EffectCall } = {}
			for _, zone in ipairs(ctx:fill_board_zones(ctx:controller())) do
				-- "Jlockheed Units": the Jlockeed-tagged Units of every set (R278, R380).
				effects[#effects + 1] = J.summon_random({
					query = { type = "Unit", tags = { "Jlockeed" } },
					lane = zone.lane,
					radiant = radiant,
				})
			end
			return effects
		end,
	}
end

local function headquarters(uses: number, radiant: boolean)
	return {
		staticFlags = { tribute = TRIBUTE_COST },
		activations = { fill(uses, radiant) },
	}
end

-- The Radiant face's second activation in a turn fills what the first left empty (R1261).
return {
	base = headquarters(BASE_USES, false),
	radiant = headquarters(RADIANT_USES, true),
}
