--!strict
local J = require("@jackioh")

local BOT_LOSER = "classicplus-019-5"

-- The declared `chance` is a percentage.
local PERCENT = 100

-- "{chance}% chance": rolled only when an enemy Unit it may attack stands (R129).
local function attacks(ctx: J.EffectContext): { string }
	local me = ctx:live_self()
	if me == nil then
		return {}
	end
	if me.zone.z ~= "field" then
		return {}
	end
	if #ctx:random_attack_targets(me.id, "enemyUnits") == 0 then
		return {}
	end
	if ctx:rng_chance(ctx:param("chance"), PERCENT) then
		return { me.id }
	end
	return {}
end

-- §3.1: lane N faces lane N. Each Bot Loser of yours with the enemy Unit across from it.
local function across_from_bot_losers(ctx: J.EffectContext)
	local controller = ctx:controller()
	local enemy = ctx:opponent_of(controller)
	local credits = {}
	for _, slot in ipairs(ctx:slots_of(controller, "units")) do
		local bot = ctx:card_at(slot.player, slot.lane)
		if bot ~= nil and bot.defId == BOT_LOSER then
			local victim = ctx:card_at(enemy, slot.lane)
			if victim ~= nil then
				credits[#credits + 1] = { victimId = victim.id, toId = bot.id }
			end
		end
	end
	return credits
end

-- A forced attack on a random enemy Unit, watched for the kills across from your Bot Losers. `transfer`
-- is the Radiant face's: the credit is the whole of it.
local function attack_of(id: string, transfer: boolean, then_: string?)
	return {
		J.with_kill_credit({
			killer = { of = "instance", instanceId = id },
			pairs = "kill_pairs",
			transfer = transfer,
			during = J.forced_attack_random({
				attacker = { of = "instance", instanceId = id },
				among = "enemyUnits",
			}),
			["then"] = then_,
		}),
	}
end

local function end_of_turn(ctx: J.EffectContext)
	return {
		J.for_each_card({ cards = "each_cards", each = "each_card" }),
	}
end

return {
	-- Base: the Bot Loser across from the kill goes Berserk.
	base = {
		endOfTurn = end_of_turn,
		each_cards = attacks,
		each_card = function(id: string)
			return attack_of(id, false, "kill_then")
		end,
		kill_pairs = across_from_bot_losers,
		kill_then = function(pair: any)
			return {
				J.go_berserk({ target = { of = "instance", instanceId = pair.toId } }),
			}
		end,
	},
	-- Radiant: the kill is credited to your Bot Loser instead.
	radiant = {
		endOfTurn = function(ctx: J.EffectContext)
			return end_of_turn(ctx)
		end,
		each_cards = attacks,
		each_card = function(id: string)
			return attack_of(id, true, nil)
		end,
		kill_pairs = across_from_bot_losers,
	},
}
