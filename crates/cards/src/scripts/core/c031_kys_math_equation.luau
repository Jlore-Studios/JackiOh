--!strict
local J = require("@jackioh")

local ID = "core-031"

-- §8 row 31: what each face adds to the times played before taking the Fibonacci number.
local FIB_OFFSET_BASE = 1
local FIB_OFFSET_RADIANT = 3

-- R429: "It costs (1) more, to a maximum of (4)."
local RETURN_COST_STEP = 1
local RETURN_COST_CAP = 4

-- R280: the formula as each face prints it — "Fib(times played + 1)" — which the preview labels
-- its number with: the first "Fib(" that a ")" closes, through that ")". Read off the catalog text,
-- so the label is always a substring of the face it names.
local function formula_in(text: string): string
	local from = 1
	while true do
		local start = string.find(text, "Fib(", from, true)
		if start == nil then
			error("#31's text names no Fib(…) formula: " .. text)
		end
		local open = start + #"Fib("
		local close = string.find(text, ")", open, true)
		if close ~= nil then
			return string.sub(text, start, close)
		end
		from = open
	end
end

-- The plays the Fib index counts: the ones the card has had, this one included. While the card is
-- resolving its play has been counted already (§10.5 step 4); anywhere else — in hand, for the
-- preview — "if it resolved now" is one more play than it has had.
local function plays_if_resolved_now(ctx: any, card: any): number
	local counted = ctx:times_played_of(card)
	if card.zone.z == "resolving" then
		return counted
	end
	return counted + 1
end

-- The damage the card deals if it resolves now — the Cry's amount and the preview's value, one
-- function so the two cannot disagree (R280). R25: `fib` clamps the index at 11, so it tops out at 89.
local function damage_for(ctx: any, card: any, radiant: boolean): number
	local offset = FIB_OFFSET_BASE
	if radiant then
		offset = FIB_OFFSET_RADIANT
	end
	return ctx:fib(plays_if_resolved_now(ctx, card) + offset)
end

local function blast(ctx: J.EffectContext)
	-- The card as it stands now, its play already counted.
	local me = ctx:live_self()
	local amount
	if me == nil then
		amount = ctx:fib(0)
	else
		amount = damage_for(ctx, me, ctx:radiant())
	end
	return {
		J.damage({ to = { of = "chosen" }, amount = amount }),
	}
end

-- R65 outside a hand: the card's own cost, `costOverride` or printed, plus `costMod`, floored at 0 —
-- with `carried`, the climb its return gives back (R429, R766), added to `costMod`.
local function own_cost(ctx: J.EffectContext, card: any, carried: number): number
	local printed = card.costOverride
	if printed == nil then
		printed = ctx:printed_cost(card.id)
	end
	return math.max(printed + card.costMod + carried, 0)
end

-- "End of turn: Return this to your hand. It costs (1) more, to a maximum of (4)." Only on the turn
-- it was played: the spell stays in the graveyard when its hand is full (`bounce` burns it back,
-- §2.4), and a card that is still there on a later turn is no longer returning, so the turn log —
-- which `startTurn` clears — is the gate.
local function return_to_hand(ctx: J.EffectContext)
	local me = ctx:live_self()
	if me == nil then
		return {}
	end
	if me.zone.z ~= "graveyard" then
		return {}
	end
	if not ctx:was_played_this_turn(me.owner, me.id) then
		return {}
	end
	-- R429, R766: the climb its earlier returns gave it, which the graveyard took off it and its
	-- return gives back (noted as its play landed it, `return_price_of`), and the return's +1 on top.
	-- R429: never above (4), so at (4) the +1 adds nothing and the climb it had is all it gets.
	local carried = ctx:return_price_of(me.id)
	local raise = math.max(math.min(RETURN_COST_STEP, RETURN_COST_CAP - own_cost(ctx, me, carried)), 0)
	-- R78: the price rides on the instance in the hand, until a graveyard takes it off again (R766).
	-- It lands only on a card that reached the hand: a full hand burns the card back to the
	-- graveyard (§2.4, R4), which is no return. `returnPrice` notes it as the climb beside the
	-- `costMod`, so the next return gives back this and nothing else.
	local price = carried + raise
	local effects: { J.EffectCall } = {
		J.bounce({ target = { of = "self" } }),
	}
	if price ~= 0 then
		effects[#effects + 1] = J.set_cost_mod({ amount = price, inHandOnly = true, returnPrice = true })
	end
	return effects
end

-- R280: "Fib(times played + 1) {n}" — the damage it deals if played now, in hand, and as the card
-- stands wherever else it may be read. It reads its own count of plays, which is the card's (§9.1):
-- `viewFor` shows it only where the card itself is shown, never on the opponent's hand.
local function preview(ctx: J.ConditionContext)
	local def = ctx:def_of(ID)
	local label
	if ctx:radiant() then
		label = formula_in(def.radiant.text)
	else
		label = formula_in(def.base.text)
	end
	return {
		{
			label = label,
			value = damage_for(ctx, ctx:self_(), ctx:radiant()),
		},
	}
end

-- The radiant face changes only the Fibonacci offset (R275), which every hook reads off the face
-- that runs, so the return clause is kept and both faces run this one table.
local base = {
	-- R429: §10.5 step 4 counts this card's plays on its instance, and step 7 notes the price it was
	-- played at for its own return (R766).
	staticFlags = { countsPlays = true, returnKeepsPrice = true },
	-- R81: the target travels in the `play` action; "target" is any unit or hero (§8 Conventions).
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = { side = "any", of = { "unit", "hero" } },
		},
	},
	cry = blast,
	endOfTurn = return_to_hand,
	preview = preview,
}

return {
	base = base,
	radiant = "base",
}
