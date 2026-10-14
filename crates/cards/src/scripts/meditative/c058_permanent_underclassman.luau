--!strict
local J = require("@jackioh")

-- The granted Death, as both faces quote it and the grant map keys it.
local BOOK_DEATH = "meditative-058#bookDeath"

-- The summon's cost: "a random (1) Unit".
local SUMMON_COST = 1

-- Summon a random (1) Unit, then grant every Unit this Cry summoned the Book Death with the face's
-- numbers (one summon, read off `summoned_so_far` as C+ #19 does). ME-GRANT (MD-D13): the grant is
-- plain data naming `bookDeath` below, which `grants::death_hook_of` runs after the Unit's own Death.
local function underclassman(radiant: boolean)
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
					data = { books = ctx:param("books") },
				}),
			}
		end,
		each_cards = function(ctx: J.EffectContext)
			return ctx:summoned_so_far()
		end,
		each_card = function(id: string, data: any)
			return {
				J.grant_ability({
					instanceId = id,
					grant = BOOK_DEATH,
					radiant = radiant,
					params = { books = data.books },
				}),
			}
		end,
		grants = {
			bookDeath = function(ctx: J.EffectContext)
				return {
					J.add_random_from_catalog({
						query = { tags = { "Book" } },
						count = ctx:grant_param("books"),
						radiant = radiant,
					}),
				}
			end,
		},
	}
end

return {
	base = underclassman(false),
	radiant = underclassman(true),
}
