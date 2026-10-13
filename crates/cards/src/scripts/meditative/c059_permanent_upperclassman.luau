--!strict
local J = require("@jackioh")

-- The granted Death, as both faces quote it and the grant map keys it.
local FUSED_BOOK_DEATH = "meditative-059#fusedBookDeath"

-- The summon's cost: "a random (4) Unit".
local SUMMON_COST = 4

-- "Fuse a random Book and AI card": one pick from each pool (MD-D14).
local FUSED = 2

-- Summon a random (4) Unit, then grant every Unit this Cry summoned the fused Book Death (one
-- summon, read off `summoned_so_far` as C+ #19 does). The granted hook fuses one pick from each
-- pool, the Book first so the fused type falls back to it, into the controller's hand at
-- (min(sum,4)) (R469).
local function upperclassman(radiant: boolean)
	return {
		cry = function(ctx: J.EffectContext)
			return {
				J.summon_random({
					query = { type = "Unit", cost = SUMMON_COST },
					radiant = radiant,
				}),
				J.for_each_card({
					cards = "each_cards",
					each = "each_card",
				}),
			}
		end,
		each_cards = function(ctx: J.EffectContext)
			return ctx:summoned_so_far()
		end,
		each_card = function(id: string)
			return {
				J.grant_ability({
					instanceId = id,
					grant = FUSED_BOOK_DEATH,
					radiant = radiant,
					params = {},
				}),
			}
		end,
		grants = {
			fusedBookDeath = function(ctx: J.EffectContext)
				return {
					J.fuse_generated({
						count = FUSED,
						pools = { { tags = { "Book" } }, { tags = { "AI" } } },
						radiant = radiant,
						handPrice = "fused",
					}),
				}
			end,
		},
	}
end

return {
	base = upperclassman(false),
	radiant = upperclassman(true),
}
