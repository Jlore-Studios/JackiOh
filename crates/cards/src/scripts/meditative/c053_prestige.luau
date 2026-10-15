--!strict
local J = require("@jackioh")

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

-- R831005 (patch v0.3.9): "The next card you play this turn becomes Radiant", a this-turn modifier its
-- next play spends (it added AI generated cards before).
local function next_play(ctx: J.EffectContext): J.EffectCall
	return J.add_player_modifier({
		mod = {
			kind = "radiantNextPlay",
			expiry = { ["until"] = "thisTurn", turn = ctx:turn().number },
		},
	})
end

-- MD-D6: the chosen card, if any, de-Radiants (R90 already allows the play with nothing to choose).
local base = {
	targets = TARGETS,
	targetChecks = TARGET_CHECKS,
	cry = function(ctx: J.EffectContext)
		return {
			J.clear_radiant({}),
			next_play(ctx),
		}
	end,
}

-- MD-D8: every Radiant card of the opponent's field (patch v0.3.9: no longer their hand and deck).
local radiant = {
	cry = function(ctx: J.EffectContext)
		local enemy = ctx:opponent_of(ctx:controller())
		local effects = {}
		for _, card in ipairs(ctx:permanents_on_field(enemy)) do
			if card.controller == enemy and card.radiant then
				effects[#effects + 1] = J.clear_radiant({ instanceId = card.id })
			end
		end
		effects[#effects + 1] = next_play(ctx)
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
