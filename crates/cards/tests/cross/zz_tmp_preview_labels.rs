use jackioh_engine::testkit::*;
const IDS: [&str; 20] = ["core-018","core-031","core-038","core-040","core-070","core-091","core-093","classic-001","classic-019","classic-043","classic-046","classic-059","classic-088","classicplus-003","classicplus-012-6","classicplus-039","classicplus-044","classicplus-045","classicplus-074","classicplus-t-ai-06"];
#[test]
fn zz_print_labels() {
    jackioh_cards::register_all();
    let mut out = serde_json::Map::new();
    for id in IDS {
        let def = jackioh_cards::card_def(id);
        let mut labels: Vec<String> = Vec::new();
        for radiant in [false, true] {
            let s = super::scenario(json!({ "p1": { "hand": [{ "def": id, "radiant": radiant }, "core-010"], "library": ["core-019","core-019","core-019"] }, "p2": { "library": ["core-019","core-019","core-019"], "field": ["core-019"] } }));
            let v = serde_json::to_value(s.view(PlayerId::P1)).unwrap();
            for c in v["you"]["hand"].as_array().cloned().unwrap_or_default() {
                for p in c["preview"].as_array().cloned().unwrap_or_default() { labels.push(format!("{}|{}", if radiant {"R"} else {"B"}, p["label"].as_str().unwrap())); }
            }
            let row = if def.type_ == CardType::Unit { "field" } else { "backrow" };
            let s = super::scenario(json!({ "p1": { row: [{ "def": id, "radiant": radiant, "faceUp": true }], "library": ["core-019","core-019","core-019"] }, "p2": { "library": ["core-019","core-019","core-019"], "field": ["core-019"] } }));
            let v = serde_json::to_value(s.view(PlayerId::P1)).unwrap();
            for key in ["units", "backrow"] {
                for c in v["you"][key].as_array().cloned().unwrap_or_default() {
                    for p in c["preview"].as_array().cloned().unwrap_or_default() { labels.push(format!("{}|{}", if radiant {"R"} else {"B"}, p["label"].as_str().unwrap())); }
                }
            }
        }
        labels.dedup();
        out.insert(id.to_string(), json!(labels));
    }
    println!("LABELS {}", serde_json::to_string(&out).unwrap());
}
