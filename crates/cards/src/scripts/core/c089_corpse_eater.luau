--!strict
local J = require("@jackioh")

-- `factor` is the whole of the radiant text: 1 gains its stats, 2 gains double (§8 Conventions).
local function feed(factor: number)
	return {
		id = "corpse-eater-feed",
		on = { "destroyed" },
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			-- `on` already narrows this; the check is what the Rust's pattern match is.
			if event.type ~= "destroyed" then
				return {}
			end
			local dead = ctx:def_of(event.defId)
			-- A destroyed Field Spell or Trap emits the same event; only units feed it.
			if dead.type ~= "Unit" then
				return {}
			end
			-- R11: a unit token ceases to exist and never reaches a graveyard, so it never died for this.
			if dead.token then
				return {}
			end
			-- R38, R89: the attack and max health the layers computed at the moment it died. R219: a
			-- gain is never a loss, so a unit #46 starved below 0 max health gives nothing rather than
			-- shrinking the Eater.
			return {
				J.buff({
					target = { of = "self" },
					attack = math.max(event.attack, 0) * factor,
					health = math.max(event.maxHealth, 0) * factor,
				}),
			}
		end,
	}
end

return {
	base = { handTriggers = { feed(1) } },
	radiant = { handTriggers = { feed(2) } },
}
