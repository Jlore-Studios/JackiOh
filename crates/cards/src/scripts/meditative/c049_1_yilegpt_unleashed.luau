--!strict
local J = require("@jackioh")

-- C+ #78 Claude's Datacenter, summoned twice into your backrow.
local DATACENTERS = "classicplus-078"
-- C #18 Glitch in the System, added Radiant at (0).
local GLITCH = "classic-018"
-- M #49.3 AI Girlfriend, summoned to animate at once.
local GIRLFRIEND = "meditative-049-3"
-- M #49.2 Yile's Virus, filling the opponent's board (or two Radiant ones, Radiant face).
local VIRUS = "meditative-049-2"

-- One clause the way the card prints it, with its current number.
local function plural(count: number, one: string, many: string): string
	return `{count} {if count == 1 then one else many}`
end

-- `count` copies of one effect.
local function times(count: number, effect: J.EffectCall): { J.EffectCall }
	local effects = {}
	for _ = 1, count do
		effects[#effects + 1] = effect
	end
	return effects
end

-- The five entries in the list's order, with the face's current numbers (MD-C23).
local function entries(ctx: J.EffectContext, radiant: boolean): { J.RandomEntryLuau }
	local datacenters = ctx:param("datacenters")
	local glitches = ctx:param("glitches")
	local setCost = ctx:param("setCost")
	local nerfs = ctx:param("nerfs")
	local viruses = ctx:param("viruses")
	local fourth: J.RandomEntryLuau
	if radiant then
		fourth = {
			label = `Summon {plural(viruses, "Radiant Yile's Virus", "Radiant Yile's Viruses")} for your opponent`,
			effects = times(viruses, J.summon({ defId = VIRUS, player = "enemy", radiant = true })),
		}
	else
		fourth = {
			label = "Fill your opponent's board with Yile's Viruses",
			effects = { J.fill_board({ defId = VIRUS, player = "enemy" }) },
		}
	end
	return {
		{
			label = `Summon {plural(datacenters, "Claude's Datacenter", "Claude's Datacenters")}`,
			effects = times(datacenters, J.summon({ defId = DATACENTERS })),
		},
		{
			label = `Add {plural(glitches, "Radiant Glitch in the System card", "Radiant Glitch in the System cards")} to your hand`,
			effects = times(glitches, J.add_to_hand({ defId = GLITCH, radiant = true, costOverride = setCost })),
		},
		{
			label = "Summon an AI Girlfriend",
			effects = { J.summon({ defId = GIRLFRIEND }) },
		},
		fourth,
		{
			label = `Nerf every card on your opponent's field, in their hand and in their deck {plural(nerfs, "time", "times")}`,
			effects = {
				J.degrade({ scope = { side = "enemy", zones = { "field", "hand", "library" } }, times = nerfs }),
			},
		},
	}
end

-- MD-C22: the ending player's unspent mana off the event — the number Bread and Butter reads — for
-- this card's owner only.
local function owner_ended_with_mana(ctx: J.EffectContext, event: J.GameEvent): boolean
	if event.type ~= "turnEnded" then
		return false
	end
	return event.player == ctx:controller() and event.unspentMana > 0
end

local function discount_for(ctx: J.EffectContext, event: J.GameEvent)
	if event.type ~= "turnEnded" then
		return {}
	end
	if event.player ~= ctx:controller() or event.unspentMana <= 0 then
		return {}
	end
	local this: J.TargetSpec = { of = "self" }
	return { J.set_cost_mod({ target = this, amount = -event.unspentMana }) }
end

-- The discount, from the hand and from the deck (R464): a `costMod` kept until a graveyard or exile
-- (R766).
local function discount_trigger(id: string)
	return {
		id = id,
		on = { "turnEnded" },
		when = owner_ended_with_mana,
		run = discount_for,
	}
end

-- The Cry is ME-PICK-N (MD-C23): the base face does `picks` different entries by the match rng, the
-- Radiant face all five in order with no roll. Start of turn: `set_radiant_random` among your
-- non-Radiant permanents on the field (R60, R242).
local function unleashed(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			if radiant then
				local effects = {}
				for _, entry in ipairs(entries(ctx, true)) do
					for _, effect in ipairs(entry.effects) do
						effects[#effects + 1] = effect
					end
				end
				return effects
			end
			return {
				J.do_random_entries({ entries = entries(ctx, false), count = ctx:param("picks") }),
			}
		end,
		startOfTurn = function(ctx: J.EffectContext)
			return { J.set_radiant_random({ zones = "field", count = ctx:param("radiants") }) }
		end,
		handTriggers = { discount_trigger("unleashed-discount-hand") },
		deckTriggers = { discount_trigger("unleashed-discount-deck") },
	}
end

return {
	base = unleashed(false),
	radiant = unleashed(true),
}
