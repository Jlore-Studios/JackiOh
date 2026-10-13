--!strict
local J = require("@jackioh")

-- R81: one damaged Unit, either side.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" }, damaged = true },
		},
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.destroy({ target = { of = "chosen" } }),
		}
	end,
}

-- §4: damage above 0.
local function damaged_enemy_units(ctx: J.EffectContext): { string }
	local ids = {}
	for _, unit in ipairs(ctx:cards_in_scope({ side = "enemy" })) do
		if unit.damage > 0 then
			ids[#ids + 1] = unit.id
		end
	end
	return ids
end

-- The list's only effect, so the ids read here are the ones it reaches.
local radiant = {
	cry = function(ctx: J.EffectContext)
		return {
			J.for_each_card({ cards = damaged_enemy_units(ctx), each = "each_card" }),
		}
	end,
	each_card = function(id: string)
		return { J.destroy({ target = { of = "instance", instanceId = id } }) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
