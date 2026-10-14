--!strict
local J = require("@jackioh")

-- Whether `instanceId` is the first card `player` played this turn, and played that once (R213:
-- "first" is the turn's log, not a flag; R826).
local function first_played(ctx: J.EffectContext, player: J.PlayerId, instanceId: string): boolean
	local played = ctx:played_ids_this_turn(player)
	if played[1] ~= instanceId then
		return false
	end
	local times = 0
	for _, id in ipairs(played) do
		if id == instanceId then
			times += 1
		end
	end
	return times == 1
end

-- On its controller's `cardResolved` (§10.5 step 7, R17), a fresh card of the definition the event
-- names (R71) for the turn's first card, at `setCost`; never its own play (R119, R826). The Radiant
-- face's copy is Radiant; the base face's never is.
local function first_card_copied(id: string, radiant: boolean)
	return {
		id = id,
		on = { "cardResolved" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardResolved" then
				return {}
			end
			local this = ctx:self_()
			if this == nil then
				return {}
			end
			if event.player ~= ctx:controller() or event.instanceId == this.id then
				return {}
			end
			if not first_played(ctx, event.player, event.instanceId) then
				return {}
			end
			return {
				J.add_to_hand({ defId = event.defId, costOverride = ctx:param("setCost"), radiant = radiant }),
			}
		end,
	}
end

return {
	base = {
		triggers = { first_card_copied("m11-first-card-copied", false) },
	},
	radiant = {
		triggers = { first_card_copied("m11r-first-card-copied", true) },
	},
}
