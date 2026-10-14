--!strict
local J = require("@jackioh")

-- The five-stack, in the order they are summoned; each goes to the unit zone of its lane (its
-- position in the list).
local FIVE_STACK = {
	"classicplus-019-1", -- Top Loser
	"classicplus-019-2", -- Jungle Loser
	"classicplus-019-3", -- Mid Loser
	"classicplus-019-4", -- Support Loser
	"classicplus-019-5", -- Bot Loser
}

local MID_LOSER = "classicplus-019-3"

local function league(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local me = ctx:controller()
			local effects = {}
			for lane, defId in ipairs(FIVE_STACK) do
				-- R688's card-specific override: a Locked lane is skipped (§8.7 row 19).
				if not ctx:is_locked(me, lane) then
					effects[#effects + 1] = J.summon({ defId = defId, lane = lane, radiant = radiant })
					if defId == MID_LOSER then
						effects[#effects + 1] = J.for_each_card({
							cards = "each_cards",
							each = "each_card",
							data = { lane = lane },
						})
					end
				end
			end
			return effects
		end,
		-- R411: the Mid Loser this list summoned into `lane`, if it is there, has its Cry triggered.
		each_cards = function(ctx: J.EffectContext, data: any): { string }
			local unit = ctx:card_at(ctx:controller(), data.lane)
			if unit == nil or unit.defId ~= MID_LOSER then
				return {}
			end
			for _, id in ipairs(ctx:summoned_so_far()) do
				if id == unit.id then
					return { unit.id }
				end
			end
			return {}
		end,
		each_card = function(id: string)
			return { J.trigger_cry({ instanceId = id }) }
		end,
	}
end

return {
	base = league(false),
	radiant = league(true),
}
