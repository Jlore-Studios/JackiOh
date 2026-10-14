--!strict
local J = require("@jackioh")

-- `memory.eaten = { defId, radiant, statsOverride?, armorOverride? }` (§10.1), plain data so the
-- state survives a replay round-trip (§9.3).
local EATEN = "eaten"

-- R81: the meal travels in the `play` action, as a `tribute` pick the client shows as one. R428: one
-- of your other Units, the unit row only.
--
-- `min = 1` is R41's "must eat if able", and R90 supplies the "if able": a board that cannot satisfy
-- it does not refuse the play, the effect fizzles ("nothing to tribute → Cry fizzles").
--
-- Deliberately no `amount`: `playChoices.tributeCostOf` would read it as §6.3's Tribute *cost*, paid
-- from the play's `tributes` list, with Sheep Tokens as 2, refusing a play the board cannot pay. This
-- card's meal is a declared target its script sacrifices itself, so the play carries no Tribute cost.
local TARGETS = {
	{
		kind = "tribute",
		min = 1,
		max = 1,
		filter = { side = "ally", of = { "unit" }, excludeSelf = true },
	},
}

-- A reader's answer, or nil: a reader with nothing to answer hands back a `null` that is not nil.
local function found(value: any): any
	if type(value) == "table" then
		return value
	end
	return nil
end

-- The meal's copy as `summon` and `fillBoard` take it: R57's flags, its §7 stats and Armor (R41).
-- `memory.eaten` stores the same shape.
local function meal_record(defId: string, radiant: boolean, stats: any, armor: any): any
	local record: { [string]: any } = { defId = defId, radiant = radiant }
	if type(stats) == "table" then
		record.statsOverride = { attack = stats.attack, health = stats.health }
	end
	if type(armor) == "number" then
		record.armorOverride = armor
	end
	return record
end

-- The named permanent off the play's selection; nil when there was nothing legal to eat. The pick is
-- read as the engine aims every chosen target (`instance_of`): on the stay the play chose it on
-- (R174) and acting on the field (§3.2, R13). A crafted Cube + Cube naming one Reborn unit eats it
-- once, and a meal a Stack play buried under the crafted card is no meal (R41).
local function meal_of(ctx: J.EffectContext): any
	local chosen: J.TargetSpec = { of = "chosen" }
	local card = found(ctx:instance_of(chosen))
	if card == nil then
		return nil
	end
	-- R428: a Unit, acting in a unit zone, never a backrow card.
	if card.zone.z ~= "field" or card.zone.row ~= "units" then
		return nil
	end
	-- "One of your other Units": ally only, and R41's "cannot eat itself".
	if card.controller ~= ctx:controller() then
		return nil
	end
	local me = found(ctx:self_())
	if me ~= nil and card.id == me.id then
		return nil
	end
	return meal_record(card.defId, card.radiant, card.statsOverride, card.armorOverride)
end

-- What this Cube remembered: on a fused card, this ingredient's own meal (R102).
local function eaten_of(ctx: J.EffectContext): any
	local stored = found(ctx:recalled(EATEN))
	if stored == nil or type(stored.defId) ~= "string" then
		return nil
	end
	return meal_record(stored.defId, stored.radiant == true, stored.statsOverride, stored.armorOverride)
end

-- Remember the meal, then eat it: the effects apply in this order, so the read happens first.
local function cry(ctx: J.EffectContext)
	local meal = meal_of(ctx)
	if meal == nil then
		return {} -- R41: nothing to tribute → the Cry fizzles.
	end
	return {
		J.remember({ key = EATEN, value = meal }),
		J.sacrifice({ target = { of = "chosen" } }),
	}
end

local base = {
	targets = TARGETS,
	cry = cry,
	death = function(ctx: J.EffectContext)
		local eaten = eaten_of(ctx)
		if eaten == nil then
			return {} -- R41: nothing eaten → Death does nothing.
		end
		-- "Summon 2 copies of it": the declared number `copies` (R386). R64 puts each in the leftmost
		-- free zone of its own row.
		local effects = {}
		for _ = 1, ctx:param("copies") do
			effects[#effects + 1] = J.summon(eaten)
		end
		return effects
	end,
}

local radiant = {
	targets = TARGETS,
	cry = cry,
	death = function(ctx: J.EffectContext)
		local eaten = eaten_of(ctx)
		if eaten == nil then
			return {}
		end
		-- R64: "fill your board" summons into every empty, unlocked unit zone left to right.
		if ctx:def_of(eaten.defId).type == "Unit" then
			return { J.fill_board(eaten) }
		end
		-- A meal that stood in a unit zone without being a Unit card (a Field Spell or Trap standing
		-- there animated, B3.1, R383) is copied as its own card: `fillBoard` makes Units only, so the
		-- board is filled with one laneless `summon` per unit zone, each placed where its own type goes
		-- (R64), and the ones with no zone left fizzle.
		local effects = {}
		for _ = 1, #ctx:slots_of(ctx:controller(), "units") do
			effects[#effects + 1] = J.summon(eaten)
		end
		return effects
	end,
}

return {
	base = base,
	radiant = radiant,
}
