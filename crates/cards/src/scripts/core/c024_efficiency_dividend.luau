--!strict
local J = require("@jackioh")

-- The three §8.2 modes, in the order the cell lists them.
local MODE_DAMAGE = "damage"
local MODE_HEAL = "heal"
local MODE_MANA = "mana"

local MODES = {
	{ kind = "mode", options = { MODE_DAMAGE, MODE_HEAL, MODE_MANA } },
}

-- R81: the target travels with the play, for the damage and heal modes alone (`forModes`). §8's
-- Conventions pick "a target" from every legal unit and hero and only an EMPTY set fizzles; a hero
-- always stands, so those modes always name one and the mana mode names none (R90). With `min: 0`
-- for every mode, a damage or heal play could name nobody and pay its X for nothing.
local TARGETS = {
	{
		kind = "target",
		min = 1,
		max = 1,
		filter = { side = "any", of = { "unit", "hero" } },
		forModes = { MODE_DAMAGE, MODE_HEAL },
	},
}

-- Patch v0.3.9 (#831): "Deal X+1" (Radiant "2X+1") damage: the declared multiple of X, plus this.
local DIVIDEND_DAMAGE_FLAT = 1

-- "Gain X mana next turn" on the base face, "X+1" on the Radiant: this much more there.
local DIVIDEND_RADIANT_MANA = 1

-- The damage and heal modes' multiples are the declared numbers `damage` and `heal` (R386): X and
-- 4X, 2X and 8X on the Radiant face, where `damage` is tuned alone (R749: the base face's "Deal X+1
-- damage" prints no multiple and is always 1X). `bonus_mana` is the mana mode's Radiant difference.
local function dividend_cry(ctx: J.EffectContext, bonus_mana: number): { J.EffectCall }
	-- The X paid, never below 0.
	local paid = math.max(ctx:x(), 0)
	local mode = ctx:chosen_options()[1]
	if mode == MODE_DAMAGE then
		local amount = ctx:param("damage") * paid + DIVIDEND_DAMAGE_FLAT
		return { J.damage({ to = { of = "chosen" }, amount = amount }) }
	end
	if mode == MODE_HEAL then
		local amount = ctx:param("heal") * paid
		local target: J.TargetSpec = { of = "chosen" }
		return { J.heal({ target = target, amount = amount }) }
	end
	if mode == MODE_MANA then
		return { J.next_turn_mana({ amount = paid + bonus_mana, player = "self" }) }
	end
	-- No mode named: nothing to resolve, and the spell still counts as played (§8 Conventions).
	return {}
end

local function end_of_turn(ctx: J.EffectContext): { J.EffectCall }
	-- The live `ctx.self`: the card as it stands now.
	local me = ctx:live_self()
	if type(me) ~= "table" then
		return {}
	end
	local played = ctx:was_played_this_turn(me.controller, me.id)
	if me.returnToHandAtEndOfTurn ~= true and not played then
		return {}
	end
	return { J.bounce({ target = { of = "self" } }) }
end

local base = {
	modes = MODES,
	targets = TARGETS,
	cry = function(ctx: J.EffectContext)
		return dividend_cry(ctx, 0)
	end,
	endOfTurn = end_of_turn,
}

local radiant = {
	modes = MODES,
	targets = TARGETS,
	cry = function(ctx: J.EffectContext)
		return dividend_cry(ctx, DIVIDEND_RADIANT_MANA)
	end,
	endOfTurn = end_of_turn,
}

return {
	base = base,
	radiant = radiant,
}
