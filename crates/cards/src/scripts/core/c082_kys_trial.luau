--!strict
local J = require("@jackioh")

-- §6.3 Discover offers three, of sets and then of numbers.
local OPTIONS = 3

-- Patch v0.3.9 (#831, R831004): KY's Trial works across every set that ships; it named the Core
-- set's numbers alone before.
local TRIAL_SETS: { J.SetName } = { "Core", "Classic", "Classic+", "Meditative" }

-- The `resume` steps the two Discovers' answers re-enter (§10.6).
local SET = "set"
local PICKED = "picked"

-- A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9). The first Discover: three of
-- the sets, drawn from the match rng.
local function cry(ctx: J.EffectContext): { J.EffectCall }
	local sets = ctx:rng_shuffle(TRIAL_SETS)
	local offered: { string } = {}
	for at = 1, math.min(OPTIONS, #sets) do
		offered[#offered + 1] = sets[at]
	end
	return {
		J.choose_mode({
			step = SET,
			options = offered,
			prompt = "KY's Trial: Discover a set",
		}),
	}
end

-- The second Discover: three of the chosen set's numbers (R247: the options are the numbers, not the
-- cards they index), the set riding the resume data.
local function set_chosen(ctx: J.EffectContext): { J.EffectCall }
	local set = ctx:chosen_options()[1]
	if set == nil then
		return {}
	end
	return {
		J.discover_from_catalog({
			step = PICKED,
			count = OPTIONS,
			-- §5.1's one pool source. Tokens and #82 itself are excluded for us — see the header.
			query = { set = set :: J.SetName },
			offer = "index",
			prompt = "KY's Trial: Discover a number",
			data = { set = set },
		}),
	}
end

-- The faces differ only in whether the card that arrives carries a `costOverride` of 0 (R65).
local function picked(ctx: J.EffectContext, costsZero: boolean): { J.EffectCall }
	local index = ctx:chosen_options()[1]
	-- §8 Conventions: an empty pick fizzles and the spell still counts as played.
	if index == nil then
		return {}
	end
	local data = ctx:data()
	local set = if type(data) == "table" then data.set else nil
	if type(set) ~= "string" then
		return {}
	end
	local def = ctx:def_by_index(set :: J.SetName, index)
	if type(def) ~= "table" then
		return {}
	end
	local args: J.AddToHandArgs = { defId = def.id, radiant = true }
	-- "It costs 0": the declared number `setCost` (R386).
	if costsZero then
		args.costOverride = ctx:param("setCost")
	end
	return { J.add_to_hand(args) }
end

local function face(costsZero: boolean)
	return {
		cry = cry,
		resume = {
			[SET] = set_chosen,
			[PICKED] = function(ctx: J.EffectContext)
				return picked(ctx, costsZero)
			end,
		},
	}
end

-- "It costs 0": the card the Discover adds, per the source note and BUILD M4-T4.
return {
	base = face(false),
	radiant = face(true),
}
