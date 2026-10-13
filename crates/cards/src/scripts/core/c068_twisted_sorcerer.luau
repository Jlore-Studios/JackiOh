--!strict
local J = require("@jackioh")

-- "If your hero is below 10", strictly: at exactly the threshold the small number is dealt. The 10 is
-- the declared number `threshold` (R386), more being better (the high number comes sooner). §10.9: a
-- hook may read state to compute an effect's arguments; it never writes, and R195's `conditionMet`
-- reads the same thing through this function.
local function hero_is_low(health: number, threshold: number): boolean
	return health < threshold
end

-- The declared number `damage` is dealt normally, `lowDamage` when the controller's hero is below the
-- threshold at resolution (R386): base "4 …, 8 if …", Radiant "8 …, 16" (R275), read off the running face.
local base = {
	-- §8 Conventions: every legal unit and hero on either side (R90: a bare `target` is units only).
	targets = {
		{
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "any",
				of = { "unit", "hero" },
			},
		},
	},
	cry = function(ctx: J.EffectContext)
		local low = hero_is_low(ctx:hero_of(ctx:controller()).health, ctx:param("threshold"))
		local amount = if low then ctx:param("lowDamage") else ctx:param("damage")
		return {
			J.damage({ to = { of = "chosen" }, amount = amount }),
		}
	end,
	-- R195: hand only — the glow says playing it now deals `lowDamage`.
	conditionMet = function(ctx: J.ConditionContext)
		return ctx:zone() == "hand" and hero_is_low(ctx:hero_of(ctx:controller()).health, ctx:param("threshold"))
	end,
}

return {
	base = base,
	radiant = "base",
}
