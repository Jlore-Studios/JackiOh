--!strict
local J = require("@jackioh")

-- §6.3 Discover offers three, and R54's roll is "1–100", which is the non-token Core set.
local OPTIONS = 3

-- B2.6: KY's Trial names the Core set's numbers, so its pool stays Core whatever else ships.
local TRIAL_SET: J.SetName = "Core"

-- The `resume` step the Discover answer re-enters (§10.6).
local PICKED = "picked"

-- A Spell's script hangs off `cry`: that is its on-resolve hook (§10.9).
local function cry(ctx: J.EffectContext): { J.EffectCall }
	return {
		J.discover_from_catalog({
			step = PICKED,
			count = OPTIONS,
			-- §5.1's one pool source. Tokens and #82 itself are excluded for us — see the header.
			query = { set = TRIAL_SET },
			-- R247: the options are the numbers, not the cards they index.
			offer = "index",
			prompt = "KY's Trial: Discover a number from 1 to 100",
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
	local def = ctx:def_by_index(TRIAL_SET, index)
	if type(def) ~= "table" then
		return {}
	end
	local args: J.AddToHandArgs = { defId = def.id, radiant = true }
	-- "It costs (0)": the declared number `setCost` (R386).
	if costsZero then
		args.costOverride = ctx:param("setCost")
	end
	return { J.add_to_hand(args) }
end

local base = {
	cry = cry,
	resume = {
		[PICKED] = function(ctx: J.EffectContext)
			return picked(ctx, false)
		end,
	},
}

-- "It costs 0": the card the Discover adds, per the source note and BUILD M4-T4.
local radiant = {
	cry = cry,
	resume = {
		[PICKED] = function(ctx: J.EffectContext)
			return picked(ctx, true)
		end,
	},
}

return {
	base = base,
	radiant = radiant,
}
