--!strict
local J = require("@jackioh")

-- R352's two Discovers: "Discover a Felinor Unit, then another".
local PICKS = 2

-- The resume step each Discover's answer re-enters (§10.6).
local PICKED = "picked"

-- The data key the picks so far travel in, from the first prompt to the second.
local PICKS_KEY = "picks"

-- One Discover of a Felinor Unit, the picks so far in its data.
local function discover(radiant: boolean, picks: { string }): J.EffectCall
	local prompt = "Discover a Felinor Unit"
	if radiant then
		prompt = "Discover a Radiant Felinor Unit"
	end
	return J.discover_from_catalog({
		step = PICKED,
		query = { tags = { "Felinor" }, type = "Unit" },
		prompt = prompt,
		-- `J.list`: the picks cross as a list even while empty, as the Rust's `[]` (the prompt's
		-- `data` is untyped, so nothing else would say an empty table is one).
		data = { [PICKS_KEY] = J.list(picks) },
	})
end

-- The picks a paused Cry has made so far, read back out of the prompt's data.
local function picks_so_far(ctx: J.EffectContext): { string }
	local data = ctx:data()
	if type(data) ~= "table" then
		return {}
	end
	local stored = data[PICKS_KEY]
	if type(stored) ~= "table" then
		return {}
	end
	local picks: { string } = {}
	for _, pick in ipairs(stored) do
		if type(pick) == "string" then
			picks[#picks + 1] = pick
		end
	end
	return picks
end

local function felinor_fuser(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return { discover(radiant, {}) }
		end,
		resume = {
			[PICKED] = function(ctx: J.EffectContext)
				local picked = ctx:chosen_options()[1]
				-- A Discover is answered with one of its options (§10.6); an answer naming
				-- none adds nothing.
				if picked == nil then
					return {}
				end
				local picks = picks_so_far(ctx)
				picks[#picks + 1] = picked
				if #picks < PICKS then
					return { discover(radiant, picks) }
				end
				-- The unit fused onto is this one, while it still stands on the field (R77).
				local me = ctx:self_()
				if type(me) ~= "table" then
					return {}
				end
				local args: J.FuseCardsArgs = { defIds = picks, targetInstanceId = me.id }
				if radiant then
					args.radiantIngredients = true
				end
				return { J.fuse_cards(args) }
			end,
		},
	}
end

return {
	base = felinor_fuser(false),
	radiant = felinor_fuser(true),
}
