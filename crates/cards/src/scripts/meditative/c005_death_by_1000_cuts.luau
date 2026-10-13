--!strict
local J = require("@jackioh")

-- One script serves both faces: the Radiant face's Lifesteal is its catalog keyword, and its larger
-- `echo` is its declared number, both read through `param`.
local base = {
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "any",
				of = { "unit" },
			},
		},
	},
	-- R802: X is `echo` times the caster's max mana, fixed as the card is played.
	echoX = function(ctx: J.HookArgs): number
		return ctx:param("echo") * ctx:max_mana_of(ctx:controller())
	end,
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "chosen" }, amount = ctx:param("damage") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
