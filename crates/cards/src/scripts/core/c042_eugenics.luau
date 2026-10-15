--!strict
local J = require("@jackioh")

-- Patch v0.3.9 (#831): "Exile all cards in your deck that cost (2) or less", on both faces, each card's
-- cost read per R65 as the clause resolves (an X card out of play is 0, an embiggen card its base
-- price, R831014). It exiled 7 random cards before.
local EUGENICS_MAX_COST = 2

-- "a 30% chance" on the base face; the Radiant face makes every card left Radiant, so nothing is left
-- to chance there and it prints no Lucky (patch v0.3.9). Percent, the integer unit the verb takes
-- (`chancePercent`).
local BASE_CHANCE_PERCENT = 30
local RADIANT_CHANCE_PERCENT = 100

-- The faces differ only in the chance (§6.1). Any Lucky given (`given_lucky_on`, R1438) still rolls
-- the base face's chance twice a card. The exile runs first and the chance rolls over what is left.
local function eugenics(chance_percent: number)
	return {
		cry = function(ctx: J.EffectContext)
			local me = ctx:live_self()
			local lucky = if me ~= nil then ctx:given_lucky_on(me) else 0
			local chance: J.RadiantChanceLuauArgs = { zone = "library", chancePercent = chance_percent }
			if lucky > 0 then
				chance.lucky = lucky
			end
			return {
				J.exile_matching({ zones = { "library" }, maxCost = EUGENICS_MAX_COST }),
				J.radiant_chance(chance),
			}
		end,
	}
end

return {
	base = eugenics(BASE_CHANCE_PERCENT),
	radiant = eugenics(RADIANT_CHANCE_PERCENT),
}
