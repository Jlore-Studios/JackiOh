--!strict
local J = require("@jackioh")

-- "AI generated cards": the R1421 AI pool.
local AI_TAG = "AI"

-- The base face's pick: a Radiant Unit or face-up backrow card on either side, or a Radiant card in
-- your hand.
local TARGETS = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = { side = "any", of = { "unit", "backrow", "hand" }, check = "radiant" },
	},
}

-- A Radiant card that is not a face-down backrow card: hidden information never shows (R440), so a
-- face-down card is not a "face-up backrow card".
local TARGET_CHECKS = {
	radiant = function(args)
		local candidate = args:candidate()
		if candidate == nil then
			return false
		end
		if not candidate.radiant then
			return false
		end
		local zone = candidate.zone
		local kind = args:card_type_of(candidate.id)
		if
			zone.z == "field"
			and zone.row == "backrow"
			and (kind == "Trap" or kind == "Field Trap")
			and candidate.faceUp ~= true
			and candidate.revealed ~= true
		then
			return false
		end
		return true
	end,
}

-- Add {cards} AI generated cards to your hand.
local function add_ai(ctx: J.EffectContext, radiant: boolean)
	return J.add_random_from_catalog({
		query = { tags = { AI_TAG } },
		count = ctx:param("cards"),
		radiant = radiant,
	})
end

-- MD-D6: the chosen card, if any, de-Radiants (R90 already allows the play with nothing to choose).
local base = {
	targets = TARGETS,
	targetChecks = TARGET_CHECKS,
	cry = function(ctx: J.EffectContext)
		return {
			J.clear_radiant({}),
			add_ai(ctx, false),
		}
	end,
}

-- MD-D8: every Radiant card of the opponent's field, hand and library — never the graveyard, whose
-- cards De-Radiant cannot reach.
local radiant = {
	cry = function(ctx: J.EffectContext)
		local enemy = ctx:opponent_of(ctx:controller())
		local effects = {}
		for _, card in ipairs(ctx:permanents_on_field(enemy)) do
			if card.controller == enemy and card.radiant then
				effects[#effects + 1] = J.clear_radiant({ instanceId = card.id })
			end
		end
		for _, zone in ipairs({ "hand", "library" }) do
			for _, card in ipairs(ctx:zone_cards(enemy, zone)) do
				if card.radiant then
					effects[#effects + 1] = J.clear_radiant({ instanceId = card.id })
				end
			end
		end
		effects[#effects + 1] = add_ai(ctx, true)
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
