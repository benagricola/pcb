use crate::common;

fn schematic(files: &[(&str, &str)]) -> pcb_sch::Schematic {
    let mut all = common::stdlib_test_files();
    for (k, v) in files {
        all.insert((*k).to_string(), (*v).to_string());
    }
    let result = common::eval_zen_raw(all, "test.zen");
    assert!(result.is_success(), "eval failed: {:?}", result.diagnostics);
    result
        .output
        .unwrap()
        .to_schematic_with_diagnostics()
        .output
        .unwrap()
}

fn find<'a>(s: &'a pcb_sch::Schematic, path: &str) -> &'a pcb_sch::InterfaceInstance {
    s.interfaces
        .iter()
        .find(|i| i.path == path)
        .unwrap_or_else(|| panic!("no {path} in {:?}", s.interfaces))
}

#[test]
#[cfg(not(target_os = "windows"))]
fn an_inferred_root_instance_is_recorded_by_its_variable_name() {
    let s = schematic(&[(
        "test.zen",
        r#"
load("@stdlib/interfaces.zen", "Spi")
DISP = Spi()
"#,
    )]);
    let disp = find(&s, "DISP");
    assert_eq!(disp.type_name, "Spi");
    assert_eq!(
        disp.members.get("CLK").map(String::as_str),
        Some("DISP_CLK")
    );
    assert_eq!(
        disp.members.keys().cloned().collect::<Vec<_>>(),
        ["CLK", "CS", "MISO", "MOSI"]
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn a_named_instance_is_recorded_by_its_variable_not_its_name() {
    let s = schematic(&[(
        "test.zen",
        r#"
load("@stdlib/interfaces.zen", "Spi")
X = Spi(name="BUS")
"#,
    )]);
    let x = find(&s, "X");
    assert_eq!(x.members.get("CLK").map(String::as_str), Some("BUS_CLK"));
    assert!(s.interfaces.iter().all(|i| i.path != "BUS"));
}

#[test]
#[cfg(not(target_os = "windows"))]
fn a_nested_diff_pair_is_an_entry_of_its_own() {
    let s = schematic(&[(
        "test.zen",
        r#"
load("@stdlib/interfaces.zen", "Csi")
CAM = Csi()
"#,
    )]);
    let clk = find(&s, "CAM.CLK");
    assert_eq!(clk.type_name, "DiffPair");
    assert_eq!(clk.members.get("P").map(String::as_str), Some("CAM_CLK_P"));
    assert_eq!(clk.members.get("N").map(String::as_str), Some("CAM_CLK_N"));
    let cam = find(&s, "CAM");
    assert_eq!(cam.type_name, "Csi");
    assert_eq!(
        cam.members.get("CLK.P").map(String::as_str),
        Some("CAM_CLK_P")
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn an_instance_passed_to_a_module_is_also_its_io_path_and_an_alias_is_one_record() {
    let s = schematic(&[
        (
            "display.zen",
            r#"
load("@stdlib/interfaces.zen", "Spi")
SPI = builtin.io(Spi)
"#,
        ),
        (
            "test.zen",
            r#"
load("@stdlib/interfaces.zen", "Spi")
Display = Module("display.zen")
DISP = Spi()
BUS = DISP
Display(name="display", SPI=DISP)
"#,
        ),
    ]);
    let io = find(&s, "display.SPI");
    assert_eq!(io.type_name, "Spi");
    assert_eq!(io.members, find(&s, "DISP").members);
    assert!(
        s.interfaces.iter().all(|i| i.path != "BUS"),
        "an alias of a recorded instance is not a second record"
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn a_binding_of_a_recorded_nested_instance_is_not_a_second_record() {
    let s = schematic(&[(
        "test.zen",
        r#"
load("@stdlib/interfaces.zen", "Csi")
CAM = Csi()
PAIR = CAM.CLK
"#,
    )]);
    assert!(
        s.interfaces.iter().all(|i| i.path != "PAIR"),
        "{:?}",
        s.interfaces
    );
    assert_eq!(
        s.interfaces
            .iter()
            .filter(|i| i.type_name == "DiffPair")
            .count(),
        5
    );
}

#[test]
#[cfg(not(target_os = "windows"))]
fn an_instance_with_no_module_level_binding_has_no_record() {
    let s = schematic(&[(
        "test.zen",
        r#"
load("@stdlib/interfaces.zen", "Spi")
BUSES = [Spi(name="S0"), Spi(name="S1")]
"#,
    )]);
    assert!(s.interfaces.is_empty(), "{:?}", s.interfaces);
}

#[test]
#[cfg(not(target_os = "windows"))]
fn an_io_bound_to_a_variable_is_recorded_once_by_its_io_name() {
    let s = schematic(&[
        (
            "display.zen",
            r#"
load("@stdlib/interfaces.zen", "Usb2")
usb = builtin.io("USB", Usb2)
"#,
        ),
        (
            "test.zen",
            r#"
load("@stdlib/interfaces.zen", "Usb2")
Display = Module("display.zen")
HOST = Usb2()
Display(name="display", USB=HOST)
"#,
        ),
    ]);
    let paths: Vec<&str> = s.interfaces.iter().map(|i| i.path.as_str()).collect();
    assert_eq!(paths, ["HOST", "HOST.D", "display.USB", "display.USB.D"]);
    assert_eq!(
        find(&s, "display.USB.D")
            .members
            .get("P")
            .map(String::as_str),
        Some("HOST_D_P")
    );
}
