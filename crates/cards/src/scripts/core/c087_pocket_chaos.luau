--!strict
local J = require("@jackioh")

local ID = "core-087"

-- R742: the gift's base cost is this much less than the cast copy's.
local GIFT_DISCOUNT = 1

-- The three things #87 swaps. The names are the engine's `SwapWhat` values, which lets `swap` with
-- no argument read the play's answer itself (§6.3 Choose one).
local SWAP_MODE = { kind = "mode", options = { "health", "board", "library" } }

-- The radiant face's second choice: hand the copy over, or keep it out of the opponent's hand.
local GIFT = "gift"
local SKIP = "skip"

local GIFT_MODE = { kind = "mode", options = { GIFT, SKIP } }

-- R742: the gift with its base cost. The copy's base cost (R65's start: its `costOverride`, else
-- its printed cost) is GIFT_DISCOUNT less than the cast copy's, so the gifts chain down
-- (4) → (3) → (2) → (1); a copy whose base cost would be (0) is never added.
local function gift(ctx: J.EffectContext): { J.EffectCall }
	local this = ctx:self_()
	if type(this) ~= "table" then
		return {}
	end
	local price = this.costOverride
	if type(price) ~= "number" then
		price = ctx:printed_cost(this.id)
	end
	local cost = price - GIFT_DISCOUNT
	if cost <= 0 then
		return {}
	end
	return { J.add_to_hand({ defId = ID, player = "enemy", costOverride = cost }) }
end

-- `radiantFace` is the whole of the radiant text: an optional gift.
local function cry(ctx: J.EffectContext, radiantFace: boolean): { J.EffectCall }
	-- Only an explicit SKIP skips: the base clause is to add it, and the radiant cell makes that
	-- optional rather than reversing it, so an unanswered gift mode still hands the copy over.
	local skipped = false
	if radiantFace then
		for _, option in ipairs(ctx:chosen_options()) do
			if option == SKIP then
				skipped = true
			end
		end
	end
	local effects: { J.EffectCall } = { J.swap({}) }
	if not skipped then
		for _, effect in ipairs(gift(ctx)) do
			effects[#effects + 1] = effect
		end
	end
	effects[#effects + 1] = J.exile({ target = { of = "self" } })
	return effects
end

local base = {
	modes = { SWAP_MODE },
	cry = function(ctx: J.EffectContext)
		return cry(ctx, false)
	end,
}

local radiant = {
	modes = { SWAP_MODE, GIFT_MODE },
	cry = function(ctx: J.EffectContext)
		return cry(ctx, true)
	end,
}

return {
	base = base,
	radiant = radiant,
}
