--!strict
local J = require("@jackioh")

-- §8.8: "Activate" is once a turn.
local USES = 1
-- The declared `chance` is a percentage.
local PERCENT = 100

-- The face's Lucky X, as Nerf and Buff have moved it (`effects/fruit.rs`'s `lucky_of` shape).
local function lucky_of(ctx: J.EffectContext): number
	local own = ctx:self_()
	if own == nil then
		return 0
	end
	for _, keyword in ipairs(ctx:numbered_keywords_on(own.id)) do
		if keyword.key == "Lucky" then
			return keyword.value
		end
	end
	return 0
end

-- One `rng.chance`, Lucky X rolling X more times and keeping a success (§6.1 Lucky): every roll is
-- drawn, as `Rng::lucky` draws them, and a success adds a random non-token card of every set (R380),
-- Radiant, hidden in your hand (R97), burned by a full hand (§2.4).
local function wish(ctx: J.EffectContext)
	local chance = ctx:param("chance")
	local lucky = lucky_of(ctx)
	local hit = ctx:rng_chance(chance, PERCENT)
	for _ = 1, math.max(lucky, 0) do
		local roll = ctx:rng_chance(chance, PERCENT)
		hit = hit or roll
	end
	if hit then
		return { J.add_random_from_catalog({ radiant = true }) }
	end
	return {}
end

-- One script serves both faces: the Radiant face's Lucky 1 is a numbered keyword on the face, read
-- as Nerf and Buff have moved it, so the base face (no Lucky) rolls once and the Radiant face twice;
-- its 0/12 is catalog data and its 12% the declared `chance`.
local base = {
	activations = {
		{
			id = "wish",
			label = "Make a wish",
			uses = USES,
			run = wish,
		},
	},
}

return {
	base = base,
	radiant = "base",
}
