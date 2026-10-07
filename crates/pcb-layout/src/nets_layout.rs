//! `nets.layout.json`: each net's type, fields and interface instances, beside the netlist, for a layout tool to
//! select nets by what the capture says they are, and the differential pairs. Keyed by the board's net names
//! (`Schematic.nets` keys, which `to_kicad_netlist` writes).
use pcb_sch::physical::{PhysicalUnitDims, PhysicalValue};
use pcb_sch::{AttributeValue, Schematic};
use rust_decimal::Decimal;
use rust_decimal::prelude::ToPrimitive;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::str::FromStr;

pub const FILE: &str = "nets.layout.json";

/// Fields whose values are lengths: a bare `m`/`mm` in them is metres, not milli-ohms.
const LENGTH_FIELDS: &[&str] = &["clearance"];

fn field_value(key: &str, v: &AttributeValue) -> Option<Value> {
    match v {
        AttributeValue::String(s) => {
            let parsed = if LENGTH_FIELDS.contains(&key) {
                PhysicalValue::parse_as(s, PhysicalUnitDims::LENGTH)
            } else {
                PhysicalValue::from_str(s)
            };
            match parsed {
                // Decimal scaling first, so 8 mil is 0.2032 and not 0.20320000000000002.
                Ok(p) if p.unit == PhysicalUnitDims::LENGTH => Some(
                    json!({"value": (p.nominal * Decimal::from(1000)).to_f64()?, "unit": "mm"}),
                ),
                // An ohm prints as an empty suffix; name it so it reads apart from a plain number.
                Ok(p) if p.unit == (PhysicalUnitDims::VOLTAGE / PhysicalUnitDims::CURRENT) => {
                    Some(json!({"value": p.nominal.to_f64()?, "unit": "Ohm"}))
                }
                Ok(p) => Some(json!({"value": p.nominal.to_f64()?, "unit": p.unit.to_string()})),
                Err(_) => Some(Value::String(s.clone())),
            }
        }
        AttributeValue::Number(n) => Some(json!(n)),
        AttributeValue::Boolean(b) => Some(json!(b)),
        _ => None,
    }
}

pub fn nets_layout_json(schematic: &Schematic, netlist: &str) -> String {
    // Net name -> (instance, type, member), sorted by instance then member.
    let mut memberships: BTreeMap<&str, Vec<(&str, &str, &str)>> = BTreeMap::new();
    let mut interfaces = BTreeMap::new();
    let mut pairs = Vec::new();
    for instance in &schematic.interfaces {
        for (member, net) in &instance.members {
            memberships.entry(net.as_str()).or_default().push((
                instance.path.as_str(),
                instance.type_name.as_str(),
                member.as_str(),
            ));
        }
        interfaces.insert(
            instance.path.as_str(),
            json!({"type": instance.type_name, "members": instance.members}),
        );
        if instance.type_name == "DiffPair"
            && let (Some(p), Some(n)) = (instance.members.get("P"), instance.members.get("N"))
        {
            pairs.push(json!({"instance": instance.path, "p": p, "n": n}));
        }
    }
    pairs.sort_by(|a, b| a["instance"].as_str().cmp(&b["instance"].as_str()));

    let mut nets = BTreeMap::new();
    for (name, net) in &schematic.nets {
        let mut fields = Map::new();
        let mut keys: Vec<_> = net.properties.keys().collect();
        keys.sort();
        for key in keys {
            if key.starts_with("symbol") || key.starts_with("__") {
                continue;
            }
            if let Some(v) = field_value(key, &net.properties[key]) {
                fields.insert(key.clone(), v);
            }
        }
        let mut member_of = memberships.remove(name.as_str()).unwrap_or_default();
        member_of.sort();
        let member_of: Vec<Value> = member_of
            .into_iter()
            .map(|(instance, ty, member)| json!({"instance": instance, "type": ty, "member": member}))
            .collect();
        nets.insert(
            name.clone(),
            json!({"type": net.kind, "fields": fields, "interfaces": member_of}),
        );
    }
    let doc = json!({
        "version": 1,
        "generator": {"name": "benagricola/pcb", "version": env!("CARGO_PKG_VERSION"), "git": env!("PCB_LAYOUT_GIT")},
        "netlist": {"file": "default.net", "sha256": hex::encode(Sha256::digest(netlist.as_bytes()))},
        "exports": ["types", "fields", "interfaces"],
        "nets": nets,
        "interfaces": interfaces,
        "pairs": pairs,
    });
    serde_json::to_string_pretty(&doc).expect("a JSON value serialises") + "\n"
}
