--!strict
local J = require("@jackioh")

-- `plague_on`: the tokens on one permanent, read off the instance itself.
local function plague_of(card: any): number
	local tokens = card.counters.plague
	if tokens == nil then
		return 0
	end
	return math.max(tokens, 0)
end

-- The match: a play `player` makes paying `cost_paid`, against this Chalice's count and its face's reach.
local function counters(me: any, controller: J.PlayerId, player: J.PlayerId, cost_paid: number, opponent_only: boolean): boolean
	if opponent_only and player == controller then
		return false
	end
	return cost_paid == plague_of(me)
end

-- The announced play's instance id when the Chalice, as it stands now, counters it (`live_self`).
local function matches(ctx: J.EffectContext, event: J.GameEvent, opponent_only: boolean): string?
	if event.type ~= "cardAnnounced" then
		return nil
	end
	local me = ctx:live_self()
	if me == nil then
		return nil
	end
	if counters(me, ctx:controller(), event.player, event.costPaid, opponent_only) then
		return event.instanceId
	end
	return nil
end

local function chalice(opponent_only: boolean)
	return {
		-- "This enters with X Plague Counters on it" (R348).
		cry = function(ctx: J.EffectContext)
			return {
				J.place_plague({ target = { of = "self" }, amount = ctx:x() }),
			}
		end,
		triggers = {
			{
				id = "plague-chalice",
				on = { "cardAnnounced" },
				when = function(ctx: J.EffectContext, event: J.GameEvent)
					return matches(ctx, event, opponent_only) ~= nil
				end,
				run = function(ctx: J.EffectContext, event: J.GameEvent)
					local instance_id = matches(ctx, event, opponent_only)
					if instance_id == nil then
						return {}
					end
					return {
						J.counter_play({ target = { of = "instance", instanceId = instance_id } }),
					}
				end,
			},
		},
		-- R667: the same match ahead of a play marks hand cards (`counteredOnPlay`).
		wouldCounter = function(ctx: any)
			return counters(ctx:self_(), ctx:controller(), ctx:player(), ctx:cost_paid(), opponent_only)
		end,
	}
end

return {
	base = chalice(false),
	radiant = chalice(true),
}
