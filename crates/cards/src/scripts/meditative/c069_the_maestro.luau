--!strict
local J = require("@jackioh")

-- "Place 1 Plague Counter on this", once per mana spent.
local PER_MANA = 1
-- "Exile … from each of your opponent's field, hand, deck and graveyard."
local ZONES: { J.ExileRandomZone } = { "field", "hand", "library", "graveyard" }

-- "Whenever you spend mana, place 1 Plague Counter on this": one placement per mana the controller
-- spent — never the opponent's, never the Maestro's own play (R212: it was not on the field yet).
local MANA_TRIGGER = {
	id = "maestro-mana-spent",
	on = { "manaSpent" },
	run = function(ctx: J.EffectContext, event: J.GameEvent)
		if event.type ~= "manaSpent" then
			return {}
		end
		if event.player ~= ctx:controller() then
			return {}
		end
		local effects = {}
		for _ = 1, event.amount do
			effects[#effects + 1] = J.place_plague({ amount = PER_MANA })
		end
		return effects
	end,
}

-- "Activate: Spend ({price}) Plague Counters: Exile {exiles} from each of your opponent's field,
-- hand, deck and graveyard." Refused below {price} counters (MD-D27).
local function exile_ability(label: string)
	return {
		id = "maestro-exile",
		label = label,
		uses = 1,
		canActivate = function(ctx: J.ConditionContext)
			return ctx:plague_on(ctx:self_().id) >= ctx:param("price")
		end,
		run = function(ctx: J.EffectContext)
			local price = ctx:param("price")
			local exiles = ctx:param("exiles")
			local effects = { J.consume_plague({ amount = price }) }
			for _, from in ipairs(ZONES) do
				effects[#effects + 1] = J.exile_random_from({ player = "enemy", from = from, count = exiles })
			end
			return effects
		end,
	}
end

return {
	base = {
		triggers = { MANA_TRIGGER },
		activations = { exile_ability("Spend (4) Plague Counters: exile 1 from each enemy zone") },
	},
	-- Two per zone on the Radiant face: its declared `exiles`.
	radiant = {
		triggers = { MANA_TRIGGER },
		activations = { exile_ability("Spend (4) Plague Counters: exile 2 from each enemy zone") },
	},
}
