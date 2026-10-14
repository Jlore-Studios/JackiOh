--!strict
local J = require("@jackioh")

-- The printed "a random Fruit": one card per start of turn.
local FRUITS = 1

-- The two faces differ only in whether the Fruit is Radiant (R276: the Radiant face's proposal).
-- "It costs (0)": the declared number `setCost` (R386).
local function fruit_tree(ctx: J.EffectContext, radiant: boolean): J.EffectCall
	local args: J.AddRandomFromCatalogArgs = {
		query = { tags = { "Fruit" } },
		count = FRUITS,
		costOverride = ctx:param("setCost"),
	}
	if radiant then
		args.radiant = true
	end
	return J.add_random_from_catalog(args)
end

local base = {
	startOfTurn = function(ctx: J.EffectContext)
		return { fruit_tree(ctx, false) }
	end,
}

local radiant = {
	startOfTurn = function(ctx: J.EffectContext)
		return { fruit_tree(ctx, true) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
