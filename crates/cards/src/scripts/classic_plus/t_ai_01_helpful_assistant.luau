--!strict
local J = require("@jackioh")

-- Radiant: "It costs (1) less."
local DISCOUNT = 1

local function discover(): { J.EffectCall }
	return {
		J.discover_from_library({
			step = "picked",
			prompt = "Discover a card from your deck",
		}),
	}
end

-- The discount as `add_to_hand`'s `costMod`, so it stacks with every other (R65); base passes 0,
-- which is leaving the field out.
local function picked(costMod: number): (ctx: J.EffectContext) -> { J.EffectCall }
	return function(ctx: J.EffectContext)
		local args: J.AddToHandArgs = { instance = { of = "chosen" } }
		if costMod ~= 0 then
			args.costMod = costMod
		end
		return { J.add_to_hand(args) }
	end
end

-- One shared `cry`: neither face reads a param (AI cards declare none). The resume steps close
-- over their face's discount, so the two faces keep their own adapters.
local base = {
	cry = function(ctx: J.EffectContext)
		return discover()
	end,
	resume = {
		picked = picked(0),
	},
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		return discover()
	end,
	resume = {
		picked = picked(-DISCOUNT),
	},
}

return {
	base = base,
	radiant = radiant,
}
