--!strict
local J = require("@jackioh")

-- One coin, Lucky X times more, heads kept if any of them lands heads (R1440's `lucky_coin`: every
-- flip is drawn, the first included).
local function lands_heads(ctx: J.EffectContext): boolean
	-- R987: the controller's Luck flips extra times beside the card's own Lucky.
	local lucky = 0
	local me = ctx:live_self()
	if type(me) == "table" and me.zone.z == "field" then
		lucky = ctx:numbered_sum(me.id, "Lucky") or 0
	end
	lucky += ctx:luck_of(ctx:controller())
	local heads = false
	for _ = 0, math.max(lucky, 0) do
		if ctx:rng_chance(1, 2) then
			heads = true
		end
	end
	return heads
end

-- "your opponent gains 1 mana next turn": the declared number `oppMana` (R386), less being better.
local base = {
	cry = function(ctx: J.EffectContext)
		if lands_heads(ctx) then
			local heads = ctx:param("heads")
			return {
				J.buff({ target = { of = "self" }, attack = heads, health = heads }),
			}
		end
		local tails = ctx:param("tails")
		return {
			J.buff({ target = { of = "self" }, attack = -tails, health = -tails }),
			J.next_turn_mana({ amount = ctx:param("oppMana"), player = "enemy" }),
		}
	end,
}

-- The same script: the Radiant face's Lucky 1 and its heads 10 are catalog data the script reads.
return {
	base = base,
	radiant = "base",
}
