--!strict
local J = require("@jackioh")

-- Both faces are the same script: §6.2's Quickdraw flag and the `heroArmor` flag. The four values
-- live in `config.HERO_ARMOR`, selected by the instance's `radiant` and `embiggened`. The card
-- declares them as `armor` and `paidArmor` (R386), which `damage::hero_armor_of` reads through
-- `params::declared_or`, so a Degrade or an Upgrade moves the Armor the price selects.
local base = {
	staticFlags = {
		quickdraw = true,
		heroArmor = true,
	},
}

return {
	base = base,
	radiant = "base",
}
