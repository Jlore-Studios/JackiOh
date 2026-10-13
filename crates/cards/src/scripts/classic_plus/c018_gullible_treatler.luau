--!strict
local J = require("@jackioh")

-- "Field Spell or Trap": a Field Trap is a Trap too (§8 Conventions).
local COUNTED: { J.CardType } = { "Field Spell", "Trap", "Field Trap" }

local function counted(card_type: J.CardType?): boolean
	for _, kind in ipairs(COUNTED) do
		if card_type == kind then
			return true
		end
	end
	return false
end

-- Whether `player`'s backrow holds a Field Spell, Trap or Field Trap, face-down ones included; an
-- animated one stands in a unit zone and is a Unit (§6.1). `ctx:card_at` reads the units row only,
-- so the backrow is read off `permanents_on_field`, which holds every backrow card; it and
-- `card_type_of` are bound on every context (#464 gap 3), and the condition context's type does
-- not list them, so the hooks pass their context as `any`.
local function controls_field_spell_or_trap(ctx: any, player: J.PlayerId): boolean
	for _, card in ipairs(ctx:permanents_on_field(player)) do
		local zone = card.zone
		if zone.z == "field" and zone.row == "backrow" and zone.player == player and counted(ctx:card_type_of(card.id)) then
			return true
		end
	end
	return false
end

-- Base: its controller controls none. Radiant: no player controls one.
local function tributes_itself(ctx: any, radiant: boolean): boolean
	local me = ctx:controller()
	local players: { J.PlayerId } = if radiant then { "p1", "p2" } else { me }
	for _, player in ipairs(players) do
		if controls_field_spell_or_trap(ctx, player) then
			return false
		end
	end
	return true
end

local function treatler(radiant: boolean)
	return {
		startOfTurn = function(ctx: J.EffectContext)
			if ctx:self_() ~= nil and tributes_itself(ctx, radiant) then
				return {
					J.sacrifice({ target = { of = "self" } }),
				}
			end
			return {}
		end,
		conditionMet = function(ctx: J.ConditionContext): boolean
			return ctx:zone() == "field" and tributes_itself(ctx, radiant)
		end,
	}
end

return {
	base = treatler(false),
	radiant = treatler(true),
}
