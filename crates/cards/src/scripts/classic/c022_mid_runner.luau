--!strict
local J = require("@jackioh")

-- "{threshold} or more mana": the one test both the Cry and the glow make.
local function enough_mana(ctx: any, mana: number): boolean
	return mana >= ctx:param("threshold")
end

local base = {
	cry = function(ctx: J.EffectContext)
		local me = ctx:live_self()
		if type(me) ~= "table" then
			return {}
		end
		-- R685: midlane is computed from the lane count, through the engine helper.
		local slot = ctx:slot_of(me.id)
		local lane = -1
		if type(slot) == "table" then
			lane = slot.lane
		end
		local in_midlane = false
		for _, mid in ipairs(ctx:midlane_lanes_of(ctx:controller())) do
			if mid == lane then
				in_midlane = true
			end
		end
		in_midlane = in_midlane and me.zone.z == "field" and me.zone.row == "units"
		local mana = ctx:mana_before_play()
		if type(mana) ~= "number" then
			mana = ctx:unspent_mana_of(ctx:controller())
		end
		local effects = {}
		if in_midlane then
			effects[#effects + 1] = J.sacrifice({ target = { of = "self" } })
		end
		if enough_mana(ctx, mana) then
			effects[#effects + 1] = J.for_each_card({ cards = "each_cards", each = "each_card" })
		end
		return effects
	end,
	-- R60: different cards, drawn as the Cry reaches this clause.
	each_cards = function(ctx: J.EffectContext)
		local permanents = ctx:cards_in_scope({ side = "enemy", rows = { "units", "backrow" } })
		local take = math.max(ctx:param("bounces"), 0)
		local ids = {}
		for _, card in ipairs(ctx:rng_shuffle(permanents)) do
			if #ids >= take then
				break
			end
			ids[#ids + 1] = card.id
		end
		return ids
	end,
	each_card = function(id: string)
		return {
			J.bounce({ target = { of = "instance", instanceId = id } }),
		}
	end,
	-- R195: in hand, whether playing it now would bounce.
	conditionMet = function(ctx: J.ConditionContext): boolean
		return ctx:zone() == "hand" and enough_mana(ctx, ctx:unspent_mana_of(ctx:controller()))
	end,
}

return {
	base = base,
	radiant = "base",
}
