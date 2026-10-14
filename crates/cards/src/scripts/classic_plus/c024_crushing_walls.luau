--!strict
local J = require("@jackioh")

-- Each player's outermost lanes (§3.1).
local function is_wall_lane(lane: number): boolean
	return lane == 1 or lane == 5
end

-- The tops of the wall zones on `side`, both rows; a Unit standing on an Ivory Tower is passed by
-- (R446, R653), the Tower itself destroyed whatever it has fused (R418).
local function in_the_walls(ctx: J.EffectContext, side: J.ScopeSide): { string }
	local ids = {}
	for _, card in ipairs(ctx:cards_in_scope({ side = side, rows = { "units", "backrow" } })) do
		if not ctx:is_carried(card.id) then
			local at = ctx:slot_of(card.id)
			if type(at) == "table" and is_wall_lane(at.lane) then
				ids[#ids + 1] = card.id
			end
		end
	end
	return ids
end

local function destroy(id: string): { J.EffectCall }
	return { J.destroy({ target = { of = "instance", instanceId = id } }) }
end

-- The list's only effect, so the ids read here are the ones it reaches.
local base = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = in_the_walls(ctx, "any"), each = "each_card" }),
		}
	end,
	each_card = destroy,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = in_the_walls(ctx, "enemy"), each = "each_card" }),
		}
	end,
	each_card = destroy,
}

return {
	base = base,
	radiant = radiant,
}
