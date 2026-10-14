--!strict
local J = require("@jackioh")

-- Where the base face keeps the survivor it drew, until the destroy reads it.
local SURVIVOR = "survivor"

-- "All Units": both sides' Units a Spell may affect, the tops of piles (§3.2), in R68's order.
local function unit_ids(ctx: J.EffectContext): { string }
	local ids = {}
	for _, unit in ipairs(ctx:cards_in_scope({ side = "any" })) do
		ids[#ids + 1] = unit.id
	end
	return ids
end

-- Every such Unit but `spared`.
local function all_but(ctx: J.EffectContext, spared: string?): { string }
	local ids = {}
	for _, id in ipairs(unit_ids(ctx)) do
		if id ~= spared then
			ids[#ids + 1] = id
		end
	end
	return ids
end

local function destroy(id: string): { J.EffectCall }
	return { J.destroy({ target = { of = "instance", instanceId = id } }) }
end

-- With one Unit or none, nothing is destroyed, so no list is read: an empty one is handed over
-- outright (an empty table crosses as a record, which `for_each_card` reads as no cards); the rest
-- are otherwise read past the survivor drawn.
local base = {
	cry = function(ctx: J.EffectContext)
		local units = unit_ids(ctx)
		local cards: J.CardsSource = if #units <= 1 then {} else "each_cards"
		return {
			-- R60: one draw of the match rng over the Units it would destroy; none, no draw (R129).
			J.remember_random({ key = SURVIVOR, options = units }),
			J.for_each_card({ cards = cards, each = "each_card" }),
		}
	end,
	each_cards = function(ctx: J.EffectContext)
		local drawn = ctx:recalled(SURVIVOR)
		return all_but(ctx, if type(drawn) == "string" then drawn else nil)
	end,
	each_card = destroy,
}

-- R81: the Radiant face's survivor, one Unit of either side.
local radiant = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit" } },
		},
	},
	-- The list's only effect, so the ids read here are the ones it reaches.
	cry = function(ctx: J.EffectContext)
		local survivor = ctx:instance_of({ of = "chosen" })
		local spared = if type(survivor) == "table" then survivor.id else nil
		return {
			J.for_each_card({ cards = all_but(ctx, spared), each = "each_card" }),
		}
	end,
	each_card = destroy,
}

return {
	base = base,
	radiant = radiant,
}
