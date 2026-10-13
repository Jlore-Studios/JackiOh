--!strict
local J = require("@jackioh")

-- §8.7: "It costs (0)".
local FREE = 0

-- The printed "a random AI generated card": one card per end of turn.
local CARDS = 1

-- §8.7: "a random AI generated card", the AI tag's ten tokens, which a pool names by asking for
-- tokens.
local function datacenter(radiant: boolean): J.EffectCall
	local args: J.AddRandomFromCatalogArgs = {
		query = { tags = { "AI" }, token = true },
		count = CARDS,
		costOverride = FREE,
	}
	if radiant then
		args.radiant = true
	end
	return J.add_random_from_catalog(args)
end

local base = {
	endOfTurn = function(ctx: J.EffectContext)
		return { datacenter(false) }
	end,
}

local radiant = {
	endOfTurn = function(ctx: J.EffectContext)
		return { datacenter(true) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
