--!strict
local J = require("@jackioh")

-- §5.1's "a random card": the non-token catalog of every set (R380), minus this card (R387).
local THIS_CARD = "core-059"

-- One card per start of turn, either face.
local CARDS = 1

-- One add. `ctx:embiggened()` is the price this Field Spell was played for (R65), read at the moment
-- the trigger resolves rather than remembered by the script, so the two faces differ only in the
-- radiant flag they put on the created card (§5.2, R74).
local function add_random_card(ctx: J.EffectContext, asRadiant: boolean): J.EffectCall
	local args: J.AddRandomFromCatalogArgs = { query = { excludeDefId = THIS_CARD }, count = CARDS }
	if asRadiant then
		args.radiant = true
	end
	-- "(paid 4: it costs 0)": the embiggen price, not the mana actually spent after discounts. The
	-- (0) is the declared number `setCost` (R386).
	if ctx:embiggened() then
		args.costOverride = ctx:param("setCost")
	end
	return J.add_random_from_catalog(args)
end

local base = {
	startOfTurn = function(ctx: J.EffectContext)
		return { add_random_card(ctx, false) }
	end,
}

local radiant = {
	startOfTurn = function(ctx: J.EffectContext)
		return { add_random_card(ctx, true) }
	end,
}

return {
	base = base,
	radiant = radiant,
}
