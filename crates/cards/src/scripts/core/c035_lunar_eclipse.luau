--!strict
local J = require("@jackioh")

-- 3 damage and −1, or 6 and −2: the declared numbers `damage` and `discount` (R386), read off the
-- face that is running, so one hook serves both faces.
local base = {
	-- R81: "target" is any unit or hero (§8 Conventions), chosen with the play.
	targets = {
		{ kind = "target", min = 1, max = 1, filter = { side = "any", of = { "unit", "hero" } } },
	},
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
			J.add_player_modifier({
				player = "self",
				mod = {
					kind = "costDiscount",
					amount = ctx:param("discount"),
					-- "the next Spell you play": Spells only, and only the next one.
					onlyType = "Spell",
					oncePerTurn = true,
					-- §2.2: cleanup takes it if no Spell used it.
					expiry = { ["until"] = "thisTurn", turn = ctx:turn().number },
				},
			}),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
