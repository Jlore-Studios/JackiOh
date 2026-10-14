--!strict
local J = require("@jackioh")

-- The face the played card resolved with. The event says so (R34, R57); a card still somewhere the
-- instance table can reach is read as a fallback for an event that does not.
local function played_radiant_flag(ctx: J.EffectContext, radiant: boolean?, instanceId: string): boolean
	if radiant ~= nil then
		return radiant
	end
	local card = ctx:find_instance(instanceId)
	if type(card) == "table" then
		return card.radiant
	end
	return false
end

-- Whether this play is one of the copier's: "you play a card" is the controller's own plays, and
-- R33 lets a stolen Field Spell read "you" as its new controller, which is `ctx:controller()`.
-- Proposed R82 excludes the play that put this Clone Machine onto the field.
local function answers(ctx: J.EffectContext, player: J.PlayerId, instanceId: string): boolean
	local me = ctx:self_()
	if type(me) ~= "table" then
		return false
	end
	if player ~= ctx:controller() then
		return false
	end
	return instanceId ~= me.id
end

-- `allRadiant` is the radiant face: every copy is Radiant; the base face keeps the played flag.
-- Three copies per play, on both faces (§8.2 row 33): the declared number `copies` (R386).
local function after_play(id: string, allRadiant: boolean)
	return {
		id = id,
		on = { "cardResolved" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			if event.type ~= "cardResolved" then
				return {}
			end
			if not answers(ctx, event.player, event.instanceId) then
				return {}
			end
			local flag = allRadiant or played_radiant_flag(ctx, event.radiant, event.instanceId)
			-- R316: the copies copy the played card, so one a full library refuses is judged by it — a
			-- Trap set face-down stays unnamed to the other player.
			return {
				J.shuffle_into({
					defId = event.defId,
					count = ctx:param("copies"),
					radiant = flag,
					copyOf = event.instanceId,
				}),
			}
		end,
	}
end

return {
	base = {
		triggers = { after_play("33-after-you-play-a-card", false) },
	},
	radiant = {
		triggers = { after_play("33r-after-you-play-a-card", true) },
	},
}
