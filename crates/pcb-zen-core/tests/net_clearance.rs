use crate::common;

fn eval_to_schematic(source: &str) -> pcb_sch::Schematic {
    let mut files = common::stdlib_test_files();
    files.insert("test.zen".to_string(), source.to_string());
    let result = common::eval_zen_raw(files, "test.zen");
    assert!(result.is_success(), "eval failed: {:?}", result.diagnostics);
    result
        .output
        .expect("output")
        .to_schematic_with_diagnostics()
        .output
        .expect("schematic")
}

#[test]
#[cfg(not(target_os = "windows"))]
fn a_net_types_clearance_is_a_property_when_set_and_none_when_not() {
    let sch = eval_to_schematic(
        r#"
load("@stdlib/interfaces.zen", "Net", "Power", "Ground", "Gpio")
load("@stdlib/units.zen", "Length")

def tie(i, net):
    Component(
        name = "R" + str(i),
        prefix = "R",
        footprint = File("@kicad-footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod"),
        pin_defs = {"P1": "1"},
        pins = {"P1": net},
    )

HV = Power("HV", clearance=Length("2mm"))
SIG = Net("SIG")
GND = Ground("GND", clearance=Length("0.5mm"))
IO = Gpio("IO", clearance=Length("0.3mm"))

check(SIG.clearance == None, "an unset clearance reads None")
check(HV.clearance != None, "a set clearance reads back")

# Net properties reach the schematic through a connected pin.
[tie(i, n) for i, n in enumerate([HV, SIG, GND, IO])]
"#,
    );
    let hv = &sch.nets["HV"];
    assert!(
        hv.properties.keys().any(|k| k.to_string() == "clearance"),
        "{:?}",
        hv.properties
    );
    assert_eq!(
        hv.properties["clearance"].string(),
        Some("2mm"),
        "stored as a PhysicalValue string"
    );
    let sig = &sch.nets["SIG"];
    assert!(
        !sig.properties.keys().any(|k| k.to_string() == "clearance"),
        "an unset clearance is not stored"
    );
    assert!(
        sch.nets["IO"]
            .properties
            .keys()
            .any(|k| k.to_string() == "clearance")
    );
    assert!(
        sch.nets["GND"]
            .properties
            .keys()
            .any(|k| k.to_string() == "clearance")
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn clearance_none_clears_an_inherited_one() {
    let sch = eval_to_schematic(
        r#"
load("@stdlib/interfaces.zen", "Power")
load("@stdlib/units.zen", "Length")
A = Power("A", clearance=Length("2mm"))
B = Power(A, clearance=None)
check(B.clearance == None, "clearance=None clears it")
Component(
    name = "R1",
    prefix = "R",
    footprint = File("@kicad-footprints/Resistor_SMD.pretty/R_0402_1005Metric.kicad_mod"),
    pin_defs = {"P1": "1"},
    pins = {"P1": B},
)
"#,
    );
    assert!(
        sch.nets
            .values()
            .all(|n| !n.properties.keys().any(|k| k.to_string() == "clearance"))
    );
}
