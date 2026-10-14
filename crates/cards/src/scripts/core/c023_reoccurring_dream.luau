--!strict
local J = require("@jackioh")

-- "30% chance" on the base face, "at 40%" on the Radiant: each roll is `rng.chance(num / den)`, the
-- same double the Rust literals 0.3 and 0.4 name.
local CHANCE_DEN = 10
local BASE_CHANCE_NUM = 3
local RADIANT_CHANCE_NUM = 4

-- §6.1: the Lucky the Radiant face prints, fixed (D14: tuning never reached this roll).
local RADIANT_LUCKY = 1

-- R60: one random non-Radiant card in the caster's hand becomes Radiant.
local function make_one_radiant(): { J.EffectCall }
	return { J.set_radiant_random({ zones = "hand", count = 1 }) }
end

-- §6.1, R1438: the Lucky the Dream rolls with: the face's printed number plus any Lucky given.
local function lucky(ctx: J.EffectContext, printed: number): number
	local me = ctx:live_self()
	return printed + (if me ~= nil then ctx:given_lucky_on(me) else 0)
end

-- R129: the roll is taken only when the pick has a hand to look in. R987: the controller's Luck
-- rolls extra times beside the card's own Lucky (R1438). Lucky X rolls X more times and keeps a
-- success, drawing every roll (no short circuit), as `rng.lucky` does.
local function dream(ctx: J.EffectContext, chance_num: number, printed: number): { J.EffectCall }
	if ctx:zone_count(ctx:controller(), "hand") == 0 then
		return {}
	end
	local extra = lucky(ctx, printed) + ctx:luck_of(ctx:controller())
	local hit = ctx:rng_chance(chance_num, CHANCE_DEN)
	for _ = 1, math.max(extra, 0) do
		local next_roll = ctx:rng_chance(chance_num, CHANCE_DEN)
		hit = hit or next_roll
	end
	if hit then
		return make_one_radiant()
	end
	return {}
end

-- §5.1 and R68: at the end of the turn it was played on, the spell goes from the graveyard back to
-- its owner's hand, and a full hand burns it (§2.4, R4) — both of which `bounce` does.
local function end_of_turn(ctx: J.EffectContext)
	local me = ctx:live_self()
	if me == nil then
		return {}
	end
	if me.returnToHandAtEndOfTurn == true or ctx:was_played_this_turn(me.controller, me.id) then
		return { J.bounce({ target = { of = "self" } }) }
	end
	return {}
end

local base = {
	cry = function(ctx: J.EffectContext)
		return dream(ctx, BASE_CHANCE_NUM, 0)
	end,
	endOfTurn = end_of_turn,
}

-- Lucky 1: roll twice and keep the success (§6.1), each roll at 40%.
local radiant = {
	cry = function(ctx: J.EffectContext)
		return dream(ctx, RADIANT_CHANCE_NUM, RADIANT_LUCKY)
	end,
	endOfTurn = end_of_turn,
}

return {
	base = base,
	radiant = radiant,
}
