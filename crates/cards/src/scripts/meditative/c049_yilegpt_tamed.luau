--!strict
local J = require("@jackioh")

-- M #49.2 Yile's Virus, summoned for the opponent.
local VIRUS = "meditative-049-2"
-- M #49.1 YileGPT Unleashed, put on the bottom of the deck.
local UNLEASHED = "meditative-049-1"

-- "Put a YileGPT Unleashed on the bottom of your deck".
local UNLEASHED_COUNT = 1

-- The unit zone across from this: its lane, read as the Cry resolves.
local function across(ctx: J.EffectContext): number?
	local me = ctx:live_self()
	if me == nil or me.zone.z ~= "field" then
		return nil
	end
	return me.zone.lane
end

-- The Cry is an aimed summon for the opponent into the zone across (§3.2, R47), which fails if it is
-- occupied; the Death puts a fresh Unleashed on the bottom of the controller's deck (ME-DECK-BOTTOM,
-- R80's cap). Both tokens are Radiant on the Radiant face.
local function tamed(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			local lane = across(ctx)
			if lane == nil then
				return {}
			end
			return {
				J.summon({ defId = VIRUS, player = "enemy", lane = lane, radiant = radiant }),
			}
		end,
		death = function(_ctx: J.EffectContext)
			return {
				J.shuffle_into({
					defId = UNLEASHED,
					count = UNLEASHED_COUNT,
					player = "self",
					radiant = radiant,
					position = "bottom",
				}),
			}
		end,
	}
end

return {
	base = tamed(false),
	radiant = tamed(true),
}
