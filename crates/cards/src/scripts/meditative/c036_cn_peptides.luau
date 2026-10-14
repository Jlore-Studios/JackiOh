--!strict
local J = require("@jackioh")

-- The card's Lucky X, printed plus given (R1438): the face's as Nerf and Buff have moved it, and any
-- Lucky given to it, plus its controller's Luck (R987). `lucky_on` reads the running card by its
-- id: the Spell resolves from its controller's resolving zone (R98), where the id finds it.
local function lucky_of(ctx: J.EffectContext): number
	local own = ctx:self_()
	local lucky = if own == nil then 0 else ctx:lucky_on(own.id)
	return lucky + ctx:luck_of(ctx:controller())
end

-- Both faces share the one script: the printed Lucky 1 and the declared `times` (7, radiant 10)
-- are what differ.
local base = {
	targets = {
		{
			-- R703: no Unit to target means no play. The default `aim: harm` stays: the destroy
			-- is the harmful branch, and a cast that targets enemies should aim it at them.
			kind = "target",
			min = 1,
			max = 1,
			filter = {
				side = "any",
				of = { "unit" },
			},
			required = true,
		},
	},
	cry = function(ctx: J.EffectContext)
		local target = ctx:instance_of({ of = "chosen" })
		if target == nil then
			return {}
		end
		local id = target.id
		local friendly = target.controller == ctx:controller()
		-- `Rng::lucky` over `coin`: one flip (`chance(1, 2)` is `coin`'s one draw under 0.5), then
		-- Lucky X more, each kept against the best so far: the Buffs on a Unit the caster controls,
		-- the destroy on an enemy one (MD-B17, R902).
		local lucky = lucky_of(ctx)
		local heads = ctx:rng_chance(1, 2)
		for _ = 1, lucky do
			local next_heads = ctx:rng_chance(1, 2)
			heads = if friendly then heads or next_heads else heads and next_heads
		end
		local it: J.TargetSpec = { of = "instance", instanceId = id }
		if heads then
			return {
				J.upgrade({ target = it, times = ctx:param("times") }),
			}
		end
		return {
			J.destroy({ target = it }),
		}
	end,
}

return {
	base = base,
	radiant = "base",
}
