--!strict
local J = require("@jackioh")

-- Destroy every permanent with neither CN nor KY, then Buff each permanent with either tag (a card
-- scope's `tags` means any of them, MD-B14). The Radiant face destroys enemy permanents only and
-- Buffs only yours; `times` is the declared number.
local function gaokao(radiant: boolean)
	local destroy_side: J.ScopeSide = if radiant then "enemy" else "any"
	local buff_side: J.ScopeSide = if radiant then "self" else "any"
	return {
		cry = function(ctx: J.EffectContext)
			local times = ctx:param("times")
			return {
				J.destroy_all({
					side = destroy_side,
					rows = { "units", "backrow" },
					notTags = { "CN", "KY" },
				}),
				J.upgrade({
					scope = { side = buff_side, zones = { "field" }, tags = { "CN", "KY" } },
					times = times,
				}),
			}
		end,
	}
end

return {
	base = gaokao(false),
	radiant = gaokao(true),
}
