use serde::{Deserialize, Serialize};


#[derive(Serialize, Deserialize, Clone, Debug, Default)]
#[cfg_attr(feature = "ts", derive(ts_rs::TS))]
#[serde(rename_all = "camelCase")]
pub struct DefenseArmorArgs {
    #[serde(default)]
    pub armor: i32,
    #[serde(default)]
    pub n: Option<i32>,
}

verbs! {
    verb!(defense_armor_aura, DefenseArmorArgs);
    verb!(defense_armor, DefenseArmorArgs);
}
