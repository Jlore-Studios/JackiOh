--!strict
local J = require("@jackioh")

-- R406: "Right-House Protector" is Core #3, the Wrong-House Attacker's twin.
local RIGHT_HOUSE = "core-003"
local WRONG_HOUSE = "classicplus-006"
-- R406: "(2 in 3)", the odds of the Right-house defender.
local RIGHT_HOUSE_CHANCES = 2
local RIGHT_HOUSE_OUT_OF = 3

-- A full board summons nothing and rolls nothing (R129). A reader with no free zone hands back a
-- `null` that is not nil, so the test is for a zone.
local function summon_one(ctx: J.EffectContext)
	if type(ctx:first_free_zone(ctx:controller(), "units")) ~= "table" then
		return {}
	end
	local defId = if ctx:rng_chance(RIGHT_HOUSE_CHANCES, RIGHT_HOUSE_OUT_OF) then RIGHT_HOUSE else WRONG_HOUSE
	return {
		J.summon({ defId = defId }),
	}
end

local function summon_both(ctx: J.EffectContext)
	return {
		J.summon({ defId = RIGHT_HOUSE }),
		J.summon({ defId = WRONG_HOUSE }),
	}
end

local base = {
	cry = summon_one,
	startOfTurn = summon_one,
}

local radiant = {
	cry = summon_both,
	startOfTurn = summon_both,
}

return {
	base = base,
	radiant = radiant,
}
