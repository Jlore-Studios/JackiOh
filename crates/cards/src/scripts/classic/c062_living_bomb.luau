--!strict
local J = require("@jackioh")

-- R400: one destroy for each permanent `player` controls with a Plague Counter on it, in R68's order.
local function destroy_plagued(ctx: J.EffectContext, player: J.PlayerId)
	local effects = {}
	for _, card in ipairs(ctx:permanents_on_field(player)) do
		if card.controller == player and ctx:plague_on(card.id) > 0 then
			effects[#effects + 1] = J.destroy({ target = { of = "instance", instanceId = card.id } })
		end
	end
	return effects
end

local function at_your_turn(ctx: J.EffectContext)
	return destroy_plagued(ctx, ctx:controller())
end

local function at_opponents_turn(ctx: J.EffectContext)
	return destroy_plagued(ctx, ctx:opponent_of(ctx:controller()))
end

-- Hooks queue at R62's start-of-turn trigger point in R68's order; one opponent-turn hook serves both
-- faces.
return {
	base = {
		startOfTurn = at_your_turn,
		startOfOpponentTurn = at_opponents_turn,
	},
	radiant = {
		startOfOpponentTurn = at_opponents_turn,
	},
}
