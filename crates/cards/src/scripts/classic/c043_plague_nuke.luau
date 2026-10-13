--!strict
local J = require("@jackioh")

-- R280: the words of the text the preview's value follows, on both faces.
local MANA_LABEL = "for each Plague Counter that was on them"

-- The Plague Counters on one card, never below 0 (`plague_on`).
local function plague_on(card: any): number
	return math.max(card.counters.plague or 0, 0)
end

-- Every Unit on the field, R68's order from `first`'s side: the tops of the unit piles (R13).
-- `active_units_of` is bound on every context (#464 gap 3); the condition context's type does not
-- list it, so the preview passes its context as `any`.
local function units_on_field(ctx: any, first: J.PlayerId): { any }
	local units = {}
	for _, player in ipairs({ first, ctx:opponent_of(first) }) do
		for _, unit in ipairs(ctx:active_units_of(player)) do
			units[#units + 1] = unit
		end
	end
	return units
end

-- The Plague Counters on every Unit on the field now: what the mana counts.
local function tokens_on_units(ctx: any, first: J.PlayerId): number
	local sum = 0
	for _, unit in ipairs(units_on_field(ctx, first)) do
		sum += plague_on(unit)
	end
	return sum
end

local function plague_nuke(resummons: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			-- Read once, before anything dies: the tokens, and the non-token Units that carry any.
			local first = ctx:turn().active
			local mana = tokens_on_units(ctx, first) * ctx:param("mana")
			local plagued: { string } = {}
			for _, unit in ipairs(units_on_field(ctx, first)) do
				if plague_on(unit) > 0 and not ctx:def_of(unit.defId).token then
					plagued[#plagued + 1] = unit.id
				end
			end
			return {
				J.destroy_all({ side = "any" }),
				J.after_state_check({ hook = "after_check", data = { mana = mana, plagued = plagued } }),
			}
		end,
		-- After the one §4.5 check (R59): the temporary mana, then on the Radiant face each plagued
		-- Unit now in a graveyard, summoned under this player's control (R174).
		after_check = function(ctx: J.EffectContext, data: any)
			local effects = { J.gain_mana({ amount = data.mana }) }
			if resummons then
				for _, id in ipairs(data.plagued) do
					local card = ctx:find_instance(id)
					if card ~= nil and card.zone.z == "graveyard" then
						effects[#effects + 1] = J.summon({
							instance = { of = "instance", instanceId = id },
							player = "self",
						})
					end
				end
			end
			return effects
		end,
		-- R280: the mana it would give now.
		preview = function(ctx: J.ConditionContext)
			return {
				{
					label = MANA_LABEL,
					value = tokens_on_units(ctx, ctx:controller()) * ctx:param("mana"),
				},
			}
		end,
	}
end

return {
	base = plague_nuke(false),
	radiant = plague_nuke(true),
}
