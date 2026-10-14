--!strict
local J = require("@jackioh")

type Zone = { player: J.PlayerId, row: J.Row }

-- The zones of this lane the face destroys in: the base face's three others, the Radiant's two enemy ones.
local function laneZones(ctx: J.EffectContext, enemyOnly: boolean): { Zone }
	local me = ctx:controller()
	local enemy = ctx:opponent_of(me)
	local zones: { Zone } = {}
	if not enemyOnly then
		zones[#zones + 1] = { player = me, row = "backrow" }
	end
	zones[#zones + 1] = { player = enemy, row = "units" }
	zones[#zones + 1] = { player = enemy, row = "backrow" }
	return zones
end

-- The card acting in one zone: the top of a unit pile, or the card in a backrow zone (`card_at`);
-- `nil` for an empty zone. A reader's `None` arrives as a null, not `nil`, so a card is a table.
local function cardAt(ctx: J.EffectContext, player: J.PlayerId, row: J.Row, lane: number): any
	if row == "units" then
		local card = ctx:card_at(player, lane)
		return if type(card) == "table" then card else nil
	end
	for _, card in ipairs(ctx:permanents_on_field(player)) do
		local zone = card.zone
		if zone.z == "field" and zone.player == player and zone.row == "backrow" and zone.lane == lane then
			return card
		end
	end
	return nil
end

-- Destroy the card acting in each of those zones of Lane Eater's lane, as the Cry resolves.
local function destroyInLane(ctx: J.EffectContext, enemyOnly: boolean): { J.EffectCall }
	local me = ctx:live_self()
	if type(me) ~= "table" or me.zone.z ~= "field" then
		return {}
	end
	local at = me.zone
	if at.row ~= "units" then
		return {}
	end
	local effects: { J.EffectCall } = {}
	for _, zone in ipairs(laneZones(ctx, enemyOnly)) do
		local card = cardAt(ctx, zone.player, zone.row, at.lane)
		if card ~= nil then
			effects[#effects + 1] = J.destroy({ target = { of = "instance", instanceId = card.id } })
		end
	end
	return effects
end

local base = {
	cry = function(ctx: J.EffectContext)
		local effects = destroyInLane(ctx, false)
		effects[#effects + 1] = J.lock_lane({})
		return effects
	end,
}

local radiant = {
	cry = function(ctx: J.EffectContext)
		local effects = destroyInLane(ctx, true)
		effects[#effects + 1] = J.lock_lane({ side = "enemy" })
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
