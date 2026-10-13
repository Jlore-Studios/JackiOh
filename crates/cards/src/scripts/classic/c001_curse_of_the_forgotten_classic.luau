--!strict
local J = require("@jackioh")

-- The label both faces print the formula under (R280).
local FORMULA = "for each card in their exile"

-- The other seat. `ConditionContext` binds no `opponent_of`, and a game has two seats.
local function opponent(player: J.PlayerId): J.PlayerId
	if player == "p1" then
		return "p2"
	end
	return "p1"
end

-- N: the damage per card times the opponent's exile size, now.
local function curse_damage(per_card: number, their_exile: number): number
	return per_card * their_exile
end

local function hit_and_draw(ctx: J.EffectContext): { J.EffectCall }
	local their_exile = ctx:zone_count(ctx:opponent_of(ctx:controller()), "exile")
	return {
		J.damage({
			to = { of = "enemyHero" },
			amount = curse_damage(ctx:param("damage"), their_exile),
		}),
		J.draw({ count = ctx:param("draw") }),
	}
end

-- The total under the formula's label (R280).
local function preview(ctx: J.ConditionContext)
	local their_exile = ctx:zone_count(opponent(ctx:controller()), "exile")
	return {
		{
			label = FORMULA,
			value = curse_damage(ctx:param("damage"), their_exile),
		},
	}
end

-- The card the Recruit will take: the newest permanent in the opponent's exile (a Spell is never
-- recruited, and a unit token is never in an exile, R11), read as the Spell resolves.
local function newest_permanent_of_theirs(ctx: J.EffectContext): any
	local exile = ctx:zone_cards(ctx:opponent_of(ctx:controller()), "exile")
	for i = #exile, 1, -1 do
		local card = exile[i]
		if ctx:def_of(card.defId).type ~= "Spell" then
			return card
		end
	end
	return nil
end

local base = {
	cry = function(ctx: J.EffectContext)
		return hit_and_draw(ctx)
	end,
	preview = preview,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		local recruited = newest_permanent_of_theirs(ctx)
		local effects = hit_and_draw(ctx)
		effects[#effects + 1] = J.recruit({ from = "exile", whose = "enemy" })
		-- Only a Unit of the recruited definition attacks, not one a card cast on the draw summoned (C+ #26).
		if recruited ~= nil and ctx:def_of(recruited.defId).type == "Unit" then
			local target: J.ForcedTarget = { spec = { of = "enemyHero" } }
			effects[#effects + 1] = J.forced_attacks({
				attackers = { side = "self", defId = recruited.defId, summonedThisScript = true },
				target = target,
			})
		end
		return effects
	end,
	preview = preview,
}

return {
	base = base,
	radiant = radiant,
}
