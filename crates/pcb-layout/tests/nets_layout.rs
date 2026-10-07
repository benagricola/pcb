use pcb_layout::nets_layout::nets_layout_json;
use pcb_sch::{AttributeValue, InterfaceInstance, Net, Schematic};
use std::collections::{BTreeMap, HashMap};

fn net(name: &str, kind: &str, props: &[(&str, &str)]) -> Net {
    Net {
        kind: kind.into(),
        id: 0,
        name: name.into(),
        ports: vec![],
        properties: props
            .iter()
            .map(|(k, v)| ((*k).to_string(), AttributeValue::String((*v).to_string())))
            .collect::<HashMap<_, _>>(),
    }
}

#[test]
fn the_sidecar_holds_each_nets_type_and_fields_in_fixed_units() {
    let mut sch = Schematic::default();
    sch.nets.insert(
        "HV".into(),
        net(
            "HV",
            "Power",
            &[
                ("voltage", "48V"),
                ("clearance", "2mm"),
                ("symbol_name", "VCC"),
            ],
        ),
    );
    sch.nets
        .insert("TAP".into(), net("TAP", "Gpio", &[("clearance", "500um")]));
    sch.nets.insert("SIG".into(), net("SIG", "Net", &[]));
    let doc: serde_json::Value =
        serde_json::from_str(&nets_layout_json(&sch, "(export (version D))\n")).unwrap();
    assert_eq!(doc["version"], 1);
    assert_eq!(doc["generator"]["name"], "benagricola/pcb");
    assert_eq!(
        doc["exports"],
        serde_json::json!(["types", "fields", "interfaces"])
    );
    assert_eq!(doc["netlist"]["file"], "default.net");
    assert_eq!(doc["netlist"]["sha256"].as_str().unwrap().len(), 64);
    assert_eq!(doc["nets"]["HV"]["type"], "Power");
    assert_eq!(
        doc["nets"]["HV"]["fields"]["clearance"],
        serde_json::json!({"value": 2.0, "unit": "mm"})
    );
    assert_eq!(doc["nets"]["HV"]["fields"]["voltage"]["value"], 48.0);
    assert_eq!(doc["nets"]["TAP"]["fields"]["clearance"]["value"], 0.5);
    assert!(doc["nets"]["HV"]["fields"].get("symbol_name").is_none());
    assert_eq!(doc["nets"]["SIG"]["fields"], serde_json::json!({}));
    assert_eq!(doc["nets"]["TAP"]["fields"]["clearance"]["value"], 0.5);
}

#[test]
fn a_bare_m_is_metres_only_in_a_length_field() {
    let mut sch = Schematic::default();
    sch.nets.insert(
        "Z".into(),
        net(
            "Z",
            "Net",
            &[("impedance", "50m"), ("clearance", "0.2032mm")],
        ),
    );
    sch.nets
        .insert("Z2".into(), net("Z2", "Net", &[("impedance", "50")]));
    let doc: serde_json::Value = serde_json::from_str(&nets_layout_json(&sch, "")).unwrap();
    assert_eq!(
        doc["nets"]["Z"]["fields"]["impedance"],
        serde_json::json!({"value": 0.05, "unit": "Ohm"})
    );
    assert_eq!(doc["nets"]["Z"]["fields"]["clearance"]["value"], 0.2032);
    assert_eq!(doc["nets"]["Z2"]["fields"]["impedance"]["unit"], "Ohm");
}

fn instance(path: &str, type_name: &str, members: &[(&str, &str)]) -> InterfaceInstance {
    InterfaceInstance {
        path: path.into(),
        type_name: type_name.into(),
        members: members
            .iter()
            .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
            .collect::<BTreeMap<_, _>>(),
    }
}

#[test]
fn the_sidecar_names_each_nets_interfaces_and_lists_the_pairs() {
    let mut sch = Schematic::default();
    for n in ["DISP_CLK", "DISP_CS", "USB_D_P", "USB_D_N", "SIG"] {
        sch.nets.insert(n.into(), net(n, "Net", &[]));
    }
    sch.interfaces = vec![
        instance("DISP", "Spi", &[("CLK", "DISP_CLK"), ("CS", "DISP_CS")]),
        instance("USB", "Usb2", &[("D.N", "USB_D_N"), ("D.P", "USB_D_P")]),
        instance("USB.D", "DiffPair", &[("N", "USB_D_N"), ("P", "USB_D_P")]),
        instance(
            "display.SPI",
            "Spi",
            &[("CLK", "DISP_CLK"), ("CS", "DISP_CS")],
        ),
    ];
    let doc: serde_json::Value = serde_json::from_str(&nets_layout_json(&sch, "")).unwrap();
    assert_eq!(
        doc["exports"],
        serde_json::json!(["types", "fields", "interfaces"])
    );
    assert_eq!(
        doc["pairs"],
        serde_json::json!([{"instance": "USB.D", "p": "USB_D_P", "n": "USB_D_N"}])
    );
    assert_eq!(
        doc["nets"]["DISP_CLK"]["interfaces"],
        serde_json::json!([
            {"instance": "DISP", "type": "Spi", "member": "CLK"},
            {"instance": "display.SPI", "type": "Spi", "member": "CLK"},
        ])
    );
    assert_eq!(
        doc["nets"]["USB_D_P"]["interfaces"],
        serde_json::json!([
            {"instance": "USB", "type": "Usb2", "member": "D.P"},
            {"instance": "USB.D", "type": "DiffPair", "member": "P"},
        ])
    );
    assert_eq!(doc["nets"]["SIG"]["interfaces"], serde_json::json!([]));
    assert_eq!(
        doc["interfaces"]["DISP"],
        serde_json::json!({"type": "Spi", "members": {"CLK": "DISP_CLK", "CS": "DISP_CS"}})
    );
    assert_eq!(doc["interfaces"]["USB.D"]["type"], "DiffPair");
}
