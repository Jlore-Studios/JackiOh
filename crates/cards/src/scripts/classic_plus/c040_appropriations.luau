--!strict
local J = require("@jackioh")

local MODES = { "Military", "Education", "Culture", "Healthcare" }

-- One face's multiples of X: attack, Books, percent, health, Armor.
type Face = { attack: number, books: number, percent: number, health: number, armor: number }

local BASE: Face = { attack = 2, books = 2, percent = 10, health = 2, armor = 1 }
local RADIANT: Face = { attack = 5, books = 5, percent = 25, health = 7, armor = 2 }
local ALL = 100

-- Your Units on the field, in your hand and in your deck (an E38 card scope).
local function units(): J.CardScope
	return { side = "self", zones = { "field", "hand", "library" }, rows = { "units" }, types = { "Unit" } }
end

local function appropriations(face: Face)
	return {
		modes = {
			{ kind = "mode", options = MODES },
		},
		cry = function(ctx: J.EffectContext)
			local x = ctx:x()
			local mode = ctx:chosen_options()[1]
			if mode == "Military" then
				return {
					J.buff_cards({ scope = units(), attack = face.attack * x }),
					J.grant_keyword_cards({ scope = units(), keyword = { kind = "Rush" } }),
				}
			end
			if mode == "Education" then
				return {
					J.shuffle_random_from_catalog({
						query = { tags = { "Book" } },
						count = face.books * x,
						radiant = true,
						enchantments = { { kind = "castOnDraw" }, { kind = "targetEnemies" } },
					}),
				}
			end
			if mode == "Culture" then
				return {
					J.radiant_chance({
						zone = { "field", "hand", "library" },
						chancePercent = math.min(ALL, face.percent * x),
					}),
				}
			end
			if mode == "Healthcare" then
				return {
					J.buff_cards({ scope = units(), health = face.health * x }),
					J.grant_keyword_cards({
						scope = units(),
						keyword = { kind = "Armor", n = face.armor * x },
					}),
				}
			end
			return {}
		end,
	}
end

return {
	base = appropriations(BASE),
	radiant = appropriations(RADIANT),
}
