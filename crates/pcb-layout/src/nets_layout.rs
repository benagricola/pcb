//! `nets.layout.json`: each net's type and fields, beside the netlist, for a layout tool to select nets by what the
//! capture says they are. Keyed by the board's net names (`Schematic.nets` keys, which `to_kicad_netlist` writes).
use pcb_sch::physical::{PhysicalUnitDims, PhysicalValue};
use pcb_sch::{AttributeValue, Schematic};
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

pub const FILE: &str = "nets.layout.json";

fn field_value(v: &AttributeValue) -> Option<Value> {
    match v {
        AttributeValue::String(s) => // Net fields are voltages, lengths and the like, so a bare `m`/`mm` is a length.
        match PhysicalValue::parse_as(s, PhysicalUnitDims::LENGTH) {
            Ok(p) if p.unit == PhysicalUnitDims::LENGTH => {
                Some(json!({"value": p.nominal.to_f64()? * 1000.0, "unit": "mm"}))
            }
            Ok(p) => Some(json!({"value": p.nominal.to_f64()?, "unit": p.unit.to_string()})),
            Err(_) => Some(Value::String(s.clone())),
        },
        AttributeValue::Number(n) => Some(json!(n)),
        AttributeValue::Boolean(b) => Some(json!(b)),
        _ => None,
    }
}

pub fn nets_layout_json(schematic: &Schematic, netlist: &str) -> String {
    let mut nets = BTreeMap::new();
    for (name, net) in &schematic.nets {
        let mut fields = Map::new();
        let mut keys: Vec<_> = net.properties.keys().collect();
        keys.sort();
        for key in keys {
            if key.starts_with("symbol") || key.starts_with("__") {
                continue;
            }
            if let Some(v) = field_value(&net.properties[key]) {
                fields.insert(key.clone(), v);
            }
        }
        nets.insert(name.clone(), json!({"type": net.kind, "fields": fields}));
    }
    let doc = json!({
        "version": 1,
        "generator": {"name": "benagricola/pcb", "version": env!("CARGO_PKG_VERSION"), "git": env!("PCB_LAYOUT_GIT")},
        "netlist": {"file": "default.net", "sha256": hex::encode(Sha256::digest(netlist.as_bytes()))},
        "exports": ["types", "fields"],
        "nets": nets,
    });
    serde_json::to_string_pretty(&doc).expect("a JSON value serialises") + "\n"
}
