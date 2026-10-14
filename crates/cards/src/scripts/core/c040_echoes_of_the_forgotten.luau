--!strict
local J = require("@jackioh")

-- How many damage each card in your exile is worth: "equal to the cards", radiant "twice the cards".
local PER_EXILED_CARD = { base = 1, radiant = 2 }

-- R280: the formula as each face prints it, which the preview labels its number with.
local FORMULA = {
	base = "the cards in your exile",
	radiant = "twice the cards in your exile",
}

-- The damage the hook deals if it runs now. R72: your own exile pile, read before anything new
-- enters it.
local function damage_now(exiled: number, per_card: number): number
	return per_card * exiled
end

-- The two faces differ only by what each card in the exile is worth.
local function echoes_of_the_forgotten(per_card: number, formula: string)
	return {
		startOfTurn = function(ctx: J.EffectContext)
			return {
				J.damage({
					to = { of = "enemyHero" },
					amount = damage_now(ctx:zone_count(ctx:controller(), "exile"), per_card),
				}),
				-- "then exile the bottom card of your library" — after the count, so it pays out next turn.
				J.exile_bottom_of_library({ player = "self" }),
			}
		end,
		preview = function(ctx: J.ConditionContext)
			return {
				{
					label = formula,
					value = damage_now(ctx:zone_count(ctx:controller(), "exile"), per_card),
				},
			}
		end,
	}
end

return {
	base = echoes_of_the_forgotten(PER_EXILED_CARD.base, FORMULA.base),
	radiant = echoes_of_the_forgotten(PER_EXILED_CARD.radiant, FORMULA.radiant),
}
