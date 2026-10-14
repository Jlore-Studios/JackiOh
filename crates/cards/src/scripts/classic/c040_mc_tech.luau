--!strict
local J = require("@jackioh")

-- The Radiant pick's resume step.
local STEAL_STEP = "steal"

-- The permanents `controller`'s opponent controls: the tops of their unit piles, then their
-- backrow. `permanents_on_field` walks units lane 1 upward then the backrow lane 1 upward, so
-- filtering to the enemy side answers exactly that order.
local function enemy_permanent_ids(ctx: J.EffectContext): { string }
	local enemy = ctx:opponent_of(ctx:controller())
	local ids: { string } = {}
	for _, card in ipairs(ctx:permanents_on_field(enemy)) do
		if card.controller == enemy then
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

-- "If your opponent controls {threshold} or more permanents": the one test the Cry and the glow
-- make. `permanents_on_field` reads at runtime on every context (`reads.rs` `shared_readers!`);
-- the generated `ConditionContext` type curates only the readers conditions reach for most
-- (`export_api.rs`: "the rest read as on the effect context"), so the glow counts through a
-- narrowly-scoped `any`.
local function enough_permanents(ctx: any, threshold: number): boolean
	local enemy = ctx:opponent_of(ctx:controller())
	local count = 0
	for _, card in ipairs(ctx:permanents_on_field(enemy)) do
		if card.controller == enemy then
			count += 1
		end
	end
	return count >= threshold
end

local function condition_met(ctx: J.ConditionContext): boolean
	return ctx:zone() == "hand" and enough_permanents(ctx, ctx:param("threshold"))
end

local base = {
	cry = function(ctx: J.EffectContext)
		if not enough_permanents(ctx, ctx:param("threshold")) then
			return {}
		end
		return {
			J.for_each_card({ cards = "each_cards", each = "each_card" }),
		}
	end,
	-- R60: one of them at random, drawn as the Cry reaches the clause: the full list through the
	-- same `Rng::shuffle`, so the draws match; the take is the first.
	each_cards = function(ctx: J.EffectContext)
		local shuffled: { string } = ctx:rng_shuffle(enemy_permanent_ids(ctx))
		if #shuffled == 0 then
			return {}
		end
		return { shuffled[1] }
	end,
	each_card = function(id: string)
		return { J.steal({ instanceId = id }) }
	end,
	conditionMet = condition_met,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		if not enough_permanents(ctx, ctx:param("threshold")) then
			return {}
		end
		return {
			J.choose_target({
				step = STEAL_STEP,
				scope = { side = "enemy", of = { "unit", "backrow" } },
				prompt = "Steal one of their permanents",
			}),
		}
	end,
	resume = {
		[STEAL_STEP] = function(ctx: J.EffectContext)
			return {
				J.steal({ target = { of = "chosen" } }),
			}
		end,
	},
	conditionMet = condition_met,
}

return {
	base = base,
	radiant = radiant,
}
