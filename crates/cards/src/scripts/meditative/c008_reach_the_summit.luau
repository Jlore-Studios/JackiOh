--!strict
local J = require("@jackioh")

-- Ascent 1's resolution prompt, answered by the damage below.
local ASCENT_DAMAGE_STEP = "ascent-damage"

-- The enemy permanents: tops of unit piles and backrow cards, face-down ones included (§3.2, R13).
local function enemy_permanents(): J.BoardScope
	return { side = "enemy", rows = { "units", "backrow" } }
end

-- R841, R60, R129: Ascent 4's picks — N distinct random enemy permanents. A pool holding N or
-- fewer is used as is with no shuffle; otherwise N are drawn from a shuffle.
local function exile_picks(ctx: J.EffectContext, count: number): { string }
	local pool = ctx:cards_in_scope(enemy_permanents())
	local ids: { string } = {}
	if #pool <= count then
		for _, card in ipairs(pool) do
			ids[#ids + 1] = card.id
		end
	else
		local take = math.max(count, 0)
		for i, card in ipairs(ctx:rng_shuffle(pool)) do
			if i > take then
				break
			end
			ids[#ids + 1] = card.id
		end
	end
	return ids
end

-- R840: the Ascent level while this play resolves — the counted plays less the current one. A
-- counted play precedes every cry, and an Echo repeat is not counted, so repeats resolve the same
-- level. Each play gains only its own level's line (R841).
local function ascent_cry(ctx: J.EffectContext)
	local me = ctx:live_self()
	if me == nil then
		return {}
	end
	local level = math.max(ctx:times_played_of(me) - 1, 0)
	if level == 0 then
		local hero: J.TargetSpec = { of = "selfHero" }
		return {
			J.heal({ target = hero, amount = ctx:param("heal") }),
		}
	elseif level == 1 then
		return {
			J.choose_target({
				step = ASCENT_DAMAGE_STEP,
				scope = { side = "any", of = { "unit", "hero" } },
				prompt = "Deal damage",
			}),
		}
	elseif level == 2 then
		return {
			J.draw({ count = ctx:param("draw") }),
		}
	elseif level == 3 then
		return {
			J.discard_random({ count = ctx:param("discards"), player = "enemy" }),
		}
	elseif level == 4 then
		local ids = exile_picks(ctx, ctx:param("exiles"))
		-- The ids go outright, read here as the Rust closure read them; an empty set leaves `cards`
		-- out, which the verb reads as no cards (an empty table would cross as a record).
		local args: J.ForEachCardArgs = { each = "each_card" }
		if #ids > 0 then
			args.cards = ids
		end
		return {
			J.for_each_card(args),
		}
	elseif level == 5 then
		return {
			J.add_random_from_catalog({
				query = {},
				count = ctx:param("adds"),
				radiant = true,
				costOverride = 0,
			}),
		}
	elseif level <= 9 then
		return {}
	end
	return {
		J.win_game({}),
	}
end

-- Ascent 1's answer: the chosen Unit or hero takes the damage.
local function ascent_damage(ctx: J.EffectContext)
	return {
		J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
	}
end

-- Ascent 4's blow: each picked permanent is exiled.
local function exile_each(id: string)
	return {
		J.exile({ target = { of = "instance", instanceId = id } }),
	}
end

-- Core #31's return without the cost raise: from the graveyard, only on the turn it was played
-- (R153, R155). A full hand burns the bounce back (§2.4).
local function return_to_hand(ctx: J.EffectContext)
	local me = ctx:live_self()
	if me == nil then
		return {}
	end
	if me.zone.z ~= "graveyard" then
		return {}
	end
	if not ctx:was_played_this_turn(me.owner, me.id) then
		return {}
	end
	return {
		J.bounce({ target = { of = "self" } }),
	}
end

-- R280: the Ascent level as the card stands, labelled with the catalog text's own words.
local function preview(ctx: J.ConditionContext)
	return {
		{
			label = "Ascent level",
			value = ctx:times_played_of(ctx:self_()),
		},
	}
end

-- R429: §10.5 step 4 counts this card's plays on its instance.
local FLAGS = { quickdraw = true, countsPlays = true }

-- R842: the computed cost — the level (base) or max(0, level − 1) (Radiant), with R65's modifiers
-- on top. The two faces differ only here.
local function face(radiant: boolean)
	return {
		staticFlags = FLAGS,
		cost = function(ctx: J.CostArgs)
			local level = ctx:times_played_of(ctx:instance())
			if radiant then
				return math.max(level - 1, 0)
			end
			return level
		end,
		cry = ascent_cry,
		resume = {
			[ASCENT_DAMAGE_STEP] = ascent_damage,
		},
		each_card = exile_each,
		endOfTurn = return_to_hand,
		preview = preview,
	}
end

return {
	base = face(false),
	radiant = face(true),
}
