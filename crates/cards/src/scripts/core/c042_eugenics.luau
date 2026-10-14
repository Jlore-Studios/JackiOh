--!strict
local J = require("@jackioh")

-- §8.2: "Exile 7 random cards from your deck", on both faces (patch v0.1.1: 8 became 7).
local EXILE_COUNT = 7

-- "a 30% chance" on the base face, "at 40%" on the Radiant: percent, the integer unit the verb
-- takes (`chancePercent`), converted to the same double the Rust literals named.
local BASE_CHANCE_PERCENT = 30
local RADIANT_CHANCE_PERCENT = 40

-- §8.2: "Lucky 1 at 40%", the Radiant face's printed Lucky.
local RADIANT_LUCKY = 1

-- The faces differ only in the chance and in the Lucky they print (§6.1): the printed number as it
-- always was (a Nerf or a Buff never reaches this roll, D14) plus any Lucky given (`given_lucky_on`,
-- R1438), so a base face given Lucky 1 rolls twice a card too. The exile runs first and the chance
-- rolls over what is left.
local function eugenics(chance_percent: number, printed_lucky: number)
	return {
		cry = function(ctx: J.EffectContext)
			local me = ctx:live_self()
			local lucky = printed_lucky + (if me ~= nil then ctx:given_lucky_on(me) else 0)
			local chance: J.RadiantChanceLuauArgs = { zone = "library", chancePercent = chance_percent }
			if lucky > 0 then
				chance.lucky = lucky
			end
			return {
				J.exile_random_from_library({ count = EXILE_COUNT }),
				J.radiant_chance(chance),
			}
		end,
	}
end

return {
	base = eugenics(BASE_CHANCE_PERCENT, 0),
	radiant = eugenics(RADIANT_CHANCE_PERCENT, RADIANT_LUCKY),
}
