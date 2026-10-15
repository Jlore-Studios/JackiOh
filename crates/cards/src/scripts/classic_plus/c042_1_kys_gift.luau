--!strict
local J = require("@jackioh")

-- "A random (4) Cost card".
local FOUR = 4

-- A Book, a KY card, a Legendary card and a (4) Cost card, in the order the text names them.
local POOLS: { J.CatalogQueryArgs } = {
	{ tags = { "Book" } },
	{ tags = { "KY" } },
	{ rarity = "Legendary" },
	{ cost = FOUR },
}

-- Activate, once a turn (R384; its controller's start of turn before patch v0.3.9), in the order
-- written. The numbers are the declared `mana`, `discards` and `heal` (R386): the Radiant face's 2, 2
-- and 10 are its own, and `ctx:radiant()` makes the cards Radiant, so one script runs both faces.
local function gift(ctx: J.EffectContext)
	local hero: J.TargetSpec = { of = "selfHero" }
	local effects = {
		J.gain_mana({ amount = ctx:param("mana") }),
		J.discard_random({ count = ctx:param("discards"), player = "enemy" }),
		J.heal({ target = hero, amount = ctx:param("heal") }),
	}
	-- "They cost (0)": the declared number `setCost` (R386).
	local cost = ctx:param("setCost")
	local radiant = ctx:radiant()
	for _, query in ipairs(POOLS) do
		effects[#effects + 1] = J.add_random_from_catalog({
			query = query,
			costOverride = cost,
			radiant = radiant,
		})
	end
	return effects
end

local base = {
	activations = {
		{
			id = "gift",
			label = "KY's Gift",
			uses = 1,
			run = gift,
		},
	},
}

return {
	base = base,
	radiant = "base",
}
