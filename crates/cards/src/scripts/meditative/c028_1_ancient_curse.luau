--!strict
local J = require("@jackioh")

-- Cast on draw (R40, R70): one hit of `damage` on its caster's own hero, Armor applying. The Radiant
-- face's Pierce is catalog data the damage pipeline reads (R346), so the faces share the script.
local base = {
	staticFlags = { castOnDraw = true },
	cry = function(ctx: J.EffectContext)
		return {
			J.damage({ to = { of = "selfHero" }, amount = ctx:param("damage") }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
