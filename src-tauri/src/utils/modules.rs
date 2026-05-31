//! Gear-module data model and parsing.
//!
//! Modules are read from the local player's `SyncContainerData` snapshot (the
//! same protobuf the live meter already decodes). Each module has up to three
//! attribute "parts" `(attribute_id, value)`. This mirrors the parser of the
//! StarResonanceAutoMod project (`module_parser.py`), but reuses the data we
//! already capture instead of sniffing packets a second time.
//!
//! The combinatorial optimizer that consumes these modules lives in
//! `crate::live::module_optimizer`.

use crate::protocol::pb::SyncContainerData;
use serde::Serialize;
use specta::Type;

/// Number of distinct module attributes (13 basic + 8 special "极").
pub const NUM_ATTRS: usize = 21;

/// Static definition of a module attribute.
pub struct AttrDef {
    pub id: i32,
    /// English label (matches StarResonanceAutoMod's English logs).
    pub name: &'static str,
    /// Chinese label (the in-game name).
    pub name_cn: &'static str,
    /// `true` for the special "极-" attributes, which use a different power
    /// curve than the basic attributes.
    pub special: bool,
}

/// All 21 module attributes, in a fixed order. The index into this table is the
/// canonical "attribute slot" used everywhere in the optimizer, so the order
/// must stay stable.
pub const ATTRS: [AttrDef; NUM_ATTRS] = [
    // --- basic (slots 0..=12) ---
    AttrDef { id: 1110, name: "Strength Boost", name_cn: "力量加持", special: false },
    AttrDef { id: 1111, name: "Agility Boost", name_cn: "敏捷加持", special: false },
    AttrDef { id: 1112, name: "Intellect Boost", name_cn: "智力加持", special: false },
    AttrDef { id: 1113, name: "Special Attack", name_cn: "特攻伤害", special: false },
    AttrDef { id: 1114, name: "Elite Strike", name_cn: "精英打击", special: false },
    AttrDef { id: 1205, name: "Healing Boost", name_cn: "特攻治疗加持", special: false },
    AttrDef { id: 1206, name: "Healing Enhance", name_cn: "专精治疗加持", special: false },
    AttrDef { id: 1307, name: "Resistance", name_cn: "抵御魔法", special: false },
    AttrDef { id: 1308, name: "Armor", name_cn: "抵御物理", special: false },
    AttrDef { id: 1407, name: "Cast Focus", name_cn: "施法专注", special: false },
    AttrDef { id: 1408, name: "Attack SPD", name_cn: "攻速专注", special: false },
    AttrDef { id: 1409, name: "Crit Focus", name_cn: "暴击专注", special: false },
    AttrDef { id: 1410, name: "Luck Focus", name_cn: "幸运专注", special: false },
    // --- special "极-" (slots 13..=20) ---
    AttrDef { id: 2104, name: "DMG Stack", name_cn: "极-伤害叠加", special: true },
    AttrDef { id: 2105, name: "Agile", name_cn: "极-灵活身法", special: true },
    AttrDef { id: 2204, name: "Life Condense", name_cn: "极-生命凝聚", special: true },
    AttrDef { id: 2205, name: "First Aid", name_cn: "极-急救措施", special: true },
    AttrDef { id: 2304, name: "Final Protection", name_cn: "极-绝境守护", special: true },
    AttrDef { id: 2404, name: "Life Wave", name_cn: "极-生命波动", special: true },
    AttrDef { id: 2405, name: "Life Steal", name_cn: "极-生命汲取", special: true },
    AttrDef { id: 2406, name: "Team Luck&Crit", name_cn: "极-全队幸暴", special: true },
];

/// Map an attribute id to its canonical slot index, if it is a known attribute.
pub fn attr_slot(id: i32) -> Option<usize> {
    ATTRS.iter().position(|a| a.id == id)
}

/// Serializable attribute metadata for the frontend attribute pickers.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AttrMeta {
    pub id: i32,
    pub name: String,
    pub name_cn: String,
    pub special: bool,
}

/// The full list of selectable attributes, in canonical order.
pub fn attribute_list() -> Vec<AttrMeta> {
    ATTRS
        .iter()
        .map(|a| AttrMeta {
            id: a.id,
            name: a.name.to_string(),
            name_cn: a.name_cn.to_string(),
            special: a.special,
        })
        .collect()
}

/// Module category, derived from the module's `config_id`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Type)]
pub enum ModuleCategory {
    Attack,
    Guardian,
    Support,
}

impl ModuleCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ModuleCategory::Attack => "Attack",
            ModuleCategory::Guardian => "Guardian",
            ModuleCategory::Support => "Support",
        }
    }
}

/// Resolve a module's English name and category from its `config_id`.
///
/// Names follow StarResonanceAutoMod's `MODULE_NAMES` table. Unknown ids fall
/// back to a generic name and infer the category from the id's family digit
/// (`(config_id / 100) % 10`: 1 = Attack, 2 = Support, 3 = Guard).
fn name_and_category(config_id: i32) -> (String, ModuleCategory) {
    let (name, category) = match config_id {
        5500101 => ("Basic Attack Module", ModuleCategory::Attack),
        5500102 => ("Advanced Attack Module", ModuleCategory::Attack),
        5500103 => ("Excellent Attack Module", ModuleCategory::Attack),
        5500104 => ("Excellent Attack Module (Preferred)", ModuleCategory::Attack),
        5500201 => ("Basic Healing Module", ModuleCategory::Support),
        5500202 => ("Advanced Healing Module", ModuleCategory::Support),
        5500203 => ("Excellent Support Module", ModuleCategory::Support),
        5500204 => ("Excellent Support Module (Preferred)", ModuleCategory::Support),
        5500301 => ("Basic Guard Module", ModuleCategory::Guardian),
        5500302 => ("Advanced Guard Module", ModuleCategory::Guardian),
        5500303 => ("Excellent Guard Module", ModuleCategory::Guardian),
        5500304 => ("Excellent Guard Module (Preferred)", ModuleCategory::Guardian),
        other => {
            let category = match (other / 100) % 10 {
                2 => ModuleCategory::Support,
                3 => ModuleCategory::Guardian,
                _ => ModuleCategory::Attack,
            };
            return (format!("Unknown Module ({other})"), category);
        }
    };
    (name.to_string(), category)
}

/// One attribute roll on a module.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModulePart {
    pub id: i32,
    pub name: String,
    pub value: i32,
    pub special: bool,
}

/// A parsed gear module.
#[derive(Debug, Clone, Serialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ModuleInfo {
    /// Unique item id. Serialized as a string because these ids routinely
    /// exceed `2^53` and would lose precision as a JS number.
    pub uuid: String,
    pub config_id: i32,
    pub name: String,
    pub category: ModuleCategory,
    pub quality: i32,
    pub parts: Vec<ModulePart>,
}

impl ModuleInfo {
    /// Dense attribute vector indexed by canonical slot (`ATTRS` order).
    pub fn attr_vector(&self) -> [i32; NUM_ATTRS] {
        let mut v = [0i32; NUM_ATTRS];
        for part in &self.parts {
            if let Some(slot) = attr_slot(part.id) {
                v[slot] += part.value;
            }
        }
        v
    }

    /// Sum of all attribute values on this module (used for prefiltering).
    pub fn total_value(&self) -> i32 {
        self.parts.iter().map(|p| p.value).sum()
    }
}

/// Parse all gear modules from the local player's container snapshot.
pub fn parse_modules(sync_data: &SyncContainerData) -> Vec<ModuleInfo> {
    let mut modules = Vec::new();

    let Some(v_data) = &sync_data.v_data else {
        return modules;
    };
    let Some(mod_data) = &v_data.r#mod else {
        return modules;
    };
    let mod_infos = &mod_data.mod_infos;
    let Some(item_package) = &v_data.item_package else {
        return modules;
    };

    for package in item_package.packages.values() {
        for (&item_key, item) in &package.items {
            let Some(mod_new_attr) = &item.mod_new_attr else {
                continue;
            };
            if mod_new_attr.mod_parts.is_empty() {
                continue;
            }

            let mod_parts = &mod_new_attr.mod_parts;
            let empty = Vec::new();
            let init_link_nums = mod_infos.get(&item_key).map_or(&empty, |mi| &mi.init_link_nums);
            let n = mod_parts.len().min(init_link_nums.len());

            let mut parts = Vec::with_capacity(n);
            for i in 0..n {
                let part_id = mod_parts[i];
                let Some(slot) = attr_slot(part_id) else {
                    continue;
                };
                let def = &ATTRS[slot];
                parts.push(ModulePart {
                    id: part_id,
                    name: def.name.to_string(),
                    value: init_link_nums[i],
                    special: def.special,
                });
            }

            if parts.is_empty() {
                continue;
            }

            let (name, category) = name_and_category(item.config_id);
            modules.push(ModuleInfo {
                uuid: item_key.to_string(),
                config_id: item.config_id,
                name,
                category,
                quality: item.quality,
                parts,
            });
        }
    }

    modules
}
