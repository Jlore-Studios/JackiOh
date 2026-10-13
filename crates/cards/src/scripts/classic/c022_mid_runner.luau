--!strict
local J = require("@jackioh")

-- "{threshold} or more mana": the one test both the Cry and the glow make.
local function enough_mana(ctx: any, mana: number): boolean
	return mana >= ctx:param("threshold")
end

-- The mana its controller had as the play began (§10.5 step 1), or, for a Cry not played now, now.
local function mana_when_played(ctx: J.EffectContext): number
	local before = ctx:mana_before_play()
	if type(before) == "number" then
		return before
	end
	return ctx:unspent_mana_of(ctx:controller())
end

local function cry(ctx: J.EffectContext): { J.EffectCall }
	local me = ctx:live_self()
	if type(me) ~= "table" then
		return {}
	end
	-- Midlane is computed from the lane count, never hardcoded.
	local lane = -1
	local at = ctx:slot_of(me.id)
	if type(at) == "table" then
		lane = at.lane
	end
	local in_midlane = false
	if me.zone.z == "field" and me.zone.row == "units" then
		for _, mid in ipairs(ctx:midlane_lanes_of(ctx:controller())) do
			if mid == lane then
				in_midlane = true
			end
		end
	end
	local bounces = ctx:param("bounces")
	local mana = mana_when_played(ctx)
	local effects: { J.EffectCall } = {}
	if in_midlane then
		effects[#effects + 1] = J.sacrifice({ target = { of = "self" } })
	end
	if enough_mana(ctx, mana) then
		effects[#effects + 1] = J.for_each_card({
			cards = "each_cards",
			each = "each_card",
			-- R60: different cards, drawn as the Cry reaches this clause: the shuffle and the
			-- take travel together in `data`, so a pause never re-rolls them (R113).
			data = { bounces = bounces },
		})
	end
	return effects
end

local base = {
	cry = cry,
	-- R60: the bounce takes that many DIFFERENT random enemy permanents (R60: unit-pile tops,
	-- R13, and backrow, face-down included), drawn with the one `Rng::shuffle`.
	each_cards = function(ctx: J.EffectContext, data: any)
		if type(data) ~= "table" or type(data.bounces) ~= "number" then
			return {}
		end
		local permanents = ctx:cards_in_scope({ side = "enemy", rows = { "units", "backrow" } })
		local shuffled = ctx:rng_shuffle(permanents)
		local ids: { string } = {}
		local want = math.max(data.bounces, 0)
		for index = 1, math.min(want, #shuffled) do
			ids[#ids + 1] = shuffled[index].id
		end
		return ids
	end,
	each_card = function(id: string)
		return { J.bounce({ target = { of = "instance", instanceId = id } }) }
	end,
	-- R195: in hand, whether playing it now would bounce.
	conditionMet = function(ctx: J.ConditionContext)
		return ctx:zone() == "hand"
			and enough_mana(ctx, ctx:unspent_mana_of(ctx:controller()))
	end,
}

-- The same script: the Radiant face is the base text on doubled stats.
return {
	base = base,
	radiant = "base",
}
