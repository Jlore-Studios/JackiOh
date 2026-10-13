--!strict
local J = require("@jackioh")

local HEADS = "headsOnTurn"

local function fire(): J.TriggerDef
	return {
		id = "ace-in-the-hole",
		on = { "turnEnded" },
		when = function(ctx: J.EffectContext, event: J.GameEvent): boolean
			if event.type ~= "turnEnded" then
				return false
			end
			if event.player ~= ctx:controller() then
				return false
			end
			return ctx:recalled(HEADS) == ctx:turn().number
		end,
		run = function(ctx: J.EffectContext, event: J.GameEvent)
			return {
				J.recruit({ count = ctx:param("recruits") }),
			}
		end,
	}
end

-- One seeded coin: a luck-based roll whose better side is heads (R1440), so Lucky X flips X more
-- coins and keeps heads if any lands heads. Each flip is the match rng's next draw
-- (`rng_chance(1, 2)` is `next() < 0.5`, exactly `coin`), and every flip is drawn even after a
-- heads, as `lucky_coin` is.
local function flip_coin(ctx: J.EffectContext): boolean
	-- R987: the controller's Luck flips extra times beside the card's own Lucky.
	local lucky = ctx:luck_of(ctx:controller())
	local me = ctx:live_self()
	if type(me) == "table" then
		lucky += ctx:lucky_on(me.id)
	end
	local heads = false
	for _ = 1, 1 + lucky do
		if ctx:rng_chance(1, 2) then
			heads = true
		end
	end
	return heads
end

local function flip(on_tails: boolean): (ctx: J.EffectContext) -> { J.EffectCall }
	return function(ctx: J.EffectContext)
		if flip_coin(ctx) then
			return {
				J.remember({ key = HEADS, value = ctx:turn().number }),
			}
		end
		if on_tails then
			return {
				J.recruit({}),
			}
		end
		return {}
	end
end

local base = {
	endOfTurn = flip(false),
	triggers = { fire() },
}

-- The Radiant face is Revealed regardless of the coin flip: the end of turn shows its face to
-- both players first, then flips as usual.
local radiant = {
	endOfTurn = function(ctx: J.EffectContext)
		local effects: { J.EffectCall } = {
			J.reveal({}),
		}
		for _, effect in ipairs(flip(true)(ctx)) do
			effects[#effects + 1] = effect
		end
		return effects
	end,
	triggers = { fire() },
}

return {
	base = base,
	radiant = radiant,
}
