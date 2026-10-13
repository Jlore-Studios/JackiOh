--!strict
local J = require("@jackioh")

-- §8.8: "Tribute 2".
local TRIBUTE_COST = 2

-- Declared `chance` is a percentage.
local PERCENT = 100

-- The face's Lucky X keyword.
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

-- CAPTURE (MD-F16): after a Unit its controller's opponent played or cast has resolved (R70, R427),
-- one roll of `chance`, the Radiant face's Lucky 1 rolling again (§6.1: every roll is drawn, a hit
-- in any of them is kept).
local base = {
	staticFlags = { tribute = TRIBUTE_COST },
	triggers = {
		{
			id = "prison-capture",
			on = { "cardResolved" },
			when = function(ctx: J.EffectContext, event: J.GameEvent)
				if event.type ~= "cardResolved" then
					return false
				end
				-- §8: "your opponent".
				if event.player == ctx:controller() then
					return false
				end
				-- §5.1: a Unit.
				if ctx:def_of(event.defId).type ~= "Unit" then
					return false
				end
				return event.permanent
			end,
			run = function(ctx: J.EffectContext, event: J.GameEvent)
				if event.type ~= "cardResolved" or not event.permanent then
					return {}
				end
				local chance = ctx:param("chance")
				local captured = ctx:rng_chance(chance, PERCENT)
				for _ = 1, math.max(lucky_of(ctx), 0) do
					local again = ctx:rng_chance(chance, PERCENT)
					captured = captured or again
				end
				if not captured then
					return {}
				end
				return {
					J.capture({ instanceId = event.instanceId }),
				}
			end,
		},
	},
}

return {
	base = base,
	radiant = "base",
}
