--!strict
local J = require("@jackioh")

-- §7's shared Rush Token, 3/3 with Rush.
local RUSH_TOKEN = "core-t-rush"

-- §10.5 step 7's event: the play this trap answers, with the cost R56 reads.
type ResolvedPlay = {
	instanceId: string,
	defId: string,
	permanent: boolean,
}

-- "When your opponent plays a (1) Cost or less card, if you have an empty unit zone" (radiant: any
-- card). Returns the play when the trap answers this event and nil when it must stay armed.
local function match_(ctx: J.EffectContext, event: J.GameEvent, any_cost: boolean): ResolvedPlay?
	if event.type ~= "cardResolved" then
		return nil
	end
	-- "the opponent plays": the trap's own controller setting off their own trap is not the trigger.
	if event.player == ctx:controller() then
		return nil
	end
	-- R56 and R70: the cost actually paid, so a cast (0) is always "1 or less".
	if not any_cost and event.costPaid > ctx:param("costLimit") then
		return nil
	end
	-- R430: "if you have an empty unit zone" — an empty, unlocked, unreserved one (R64).
	if #ctx:open_zones(ctx:controller(), "units") == 0 then
		return nil
	end
	return {
		instanceId = event.instanceId,
		defId = event.defId,
		permanent = event.permanent,
	}
end

-- "summon 2 Rush Tokens" / "fill your board with Rush Tokens"; then, if it was a Unit, "they attack
-- it" with only the tokens this trap summoned (`summonedThisScript`). The Unit test reads the def, so
-- a unit that died in its own resolution still counts; "it" is the played card's stay on the field
-- (R174), so once `permanent` says that has ended nothing attacks.
local function tokens_and_attack(ctx: J.EffectContext, played: ResolvedPlay, fill: boolean): { J.EffectCall }
	local tokens: { J.EffectCall }
	if fill then
		tokens = { J.fill_board({ defId = RUSH_TOKEN }) }
	else
		tokens = {
			J.summon({ defId = RUSH_TOKEN }),
			J.summon({ defId = RUSH_TOKEN }),
		}
	end
	if ctx:def_of(played.defId).type ~= "Unit" or not played.permanent then
		return tokens
	end
	tokens[#tokens + 1] = J.forced_attacks({
		attackers = { side = "self", defId = RUSH_TOKEN, summonedThisScript = true },
		target = { instanceId = played.instanceId },
	})
	return tokens
end

-- One face's trigger. The whole condition lives in the `when` predicate (R99, R61): `run` is reached
-- only once the trap really is firing, so the nil branch below is narrowing and never a decision.
local function honeypot(any_cost: boolean, fill: boolean)
	return {
		id = "bear-honeypot",
		on = { "cardResolved" },
		when = function(ctx: J.EffectContext, event: J.GameEvent)
			return match_(ctx, event, any_cost) ~= nil
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			local played = match_(ctx, event, any_cost)
			if played == nil then
				return {}
			end
			return tokens_and_attack(ctx, played, fill)
		end,
	}
end

-- R662, R430: armed while its controller has an open unit zone for the tokens.
local function condition_met(ctx: J.ConditionContext): boolean
	return ctx:zone() == "field" and #ctx:open_zones(ctx:controller(), "units") > 0
end

return {
	base = {
		triggers = { honeypot(false, false) },
		conditionMet = condition_met,
	},
	radiant = {
		triggers = { honeypot(true, true) },
		conditionMet = condition_met,
	},
}
