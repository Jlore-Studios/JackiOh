--!strict
local J = require("@jackioh")

-- §7: the Bread Token definition; its X/X comes from `statsOverride`, never from the def.
local BREAD_TOKEN = "core-t-bread"

-- The other seat. `ConditionContext` binds no `opponent_of`, and a game has two seats.
local function opponent(player: J.PlayerId): J.PlayerId
	if player == "p1" then
		return "p2"
	end
	return "p1"
end

-- The ending player's unspent mana, straight off the event. Any other event type is not this
-- trigger's (`on` already guarantees `turnEnded`), and a negative pool cannot happen, so both read
-- as "nothing unspent".
local function unspent_on(event: J.GameEvent): number
	if event.type == "turnEnded" then
		return math.max(event.unspentMana, 0)
	end
	return 0
end

-- R280: the formula as the face prints it, which the preview labels its number with — X on the base
-- face, `multiplier` × the unspent mana on the Radiant one, with the multiplier as it stands.
local function formula(radiant: boolean, multiplier: number): string
	if radiant then
		return "X = " .. tostring(multiplier) .. " \u{00d7} that player's unspent mana"
	end
	return "X = that player's unspent mana"
end

-- The active player: `your_turn` names it without the card reading `state.active` (README §1).
local function active_player(ctx: J.ConditionContext): J.PlayerId
	if ctx:your_turn() then
		return ctx:controller()
	end
	return opponent(ctx:controller())
end

-- The same on both faces: `multiplier` is §8's X (base) or 3X (radiant), a declared number (R386)
-- tuned on the Radiant face only (R749: the base face prints no number, always 1). The condition is
-- written twice: `traps.rs` filters on `when`, `run_queued_trigger` ignores it and takes an empty
-- effect list as "not met", so either way 0 unspent summons nothing and the trap stays armed.
local base = {
	triggers = {
		{
			id = "bread-and-butter",
			on = { "turnEnded" },
			when = function(ctx: J.EffectContext, event: J.GameEvent)
				return unspent_on(event) > 0
			end,
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				local x = unspent_on(event) * ctx:param("multiplier")
				if x <= 0 then
					return {}
				end
				return {
					J.summon({
						defId = BREAD_TOKEN,
						-- R52: the trap's controller, whoever ended the turn.
						player = "self",
						-- §7, R37: X/X on a card printed 0/0.
						statsOverride = { attack = x, health = x },
						-- §7: the radiant face prints "Armor X", the same X; carried now because nothing later knows X.
						armorOverride = x,
					}),
				}
			end,
		},
	},
	-- R280: the Bread Token's X if the active player ended the turn now.
	preview = function(ctx: J.ConditionContext)
		local multiplier = ctx:param("multiplier")
		return {
			{
				label = formula(ctx:radiant(), multiplier),
				value = ctx:unspent_mana_of(active_player(ctx)) * multiplier,
			},
		}
	end,
	-- R662: "ends a turn with unspent mana", asked of the turn as it stands; the same on both faces.
	conditionMet = function(ctx: J.ConditionContext)
		if ctx:zone() ~= "field" then
			return false
		end
		return ctx:unspent_mana_of(active_player(ctx)) > 0
	end,
}

return {
	base = base,
	radiant = "base",
}
