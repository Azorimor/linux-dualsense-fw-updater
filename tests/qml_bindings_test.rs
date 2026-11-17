use std::collections::BTreeSet;
use std::path::PathBuf;

fn repo_path(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(rel)
}

fn snake_to_camel(name: &str) -> String {
    let mut out = String::with_capacity(name.len());
    let mut upper_next = false;
    for ch in name.chars() {
        if ch == '_' {
            upper_next = true;
        } else if upper_next {
            out.push(ch.to_ascii_uppercase());
            upper_next = false;
        } else {
            out.push(ch);
        }
    }
    out
}

fn extract_bridge_members(bridge_src: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut properties = BTreeSet::new();
    let mut invokables = BTreeSet::new();

    let mut expect_invokable_fn = false;
    for raw_line in bridge_src.lines() {
        let line = raw_line.trim();
        if let Some(rest) = line.strip_prefix("#[qproperty(")
            && let Some(inner) = rest.strip_suffix(")]")
        {
            let parts: Vec<&str> = inner.split(',').map(str::trim).collect();
            if parts.len() >= 2 {
                properties.insert(snake_to_camel(parts[1]));
            }
            continue;
        }

        if line == "#[qinvokable]" {
            expect_invokable_fn = true;
            continue;
        }

        if expect_invokable_fn {
            expect_invokable_fn = false;
            if let Some(after_fn) = line.strip_prefix("fn ")
                && let Some(name) = after_fn.split('(').next()
            {
                invokables.insert(snake_to_camel(name.trim()));
            }
        }
    }

    (properties, invokables)
}

fn extract_updater_usages(qml_src: &str) -> (BTreeSet<String>, BTreeSet<String>) {
    let mut props_used = BTreeSet::new();
    let mut methods_used = BTreeSet::new();
    let bytes = qml_src.as_bytes();
    let needle = b"updater.";

    let mut i = 0;
    while i + needle.len() < bytes.len() {
        if &bytes[i..i + needle.len()] == needle {
            let prev_ok = i == 0 || !(bytes[i - 1].is_ascii_alphanumeric() || bytes[i - 1] == b'_');
            if prev_ok {
                let start = i + needle.len();
                let mut end = start;
                while end < bytes.len()
                    && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_')
                {
                    end += 1;
                }
                if end > start {
                    let ident = String::from_utf8_lossy(&bytes[start..end]).into_owned();
                    let mut lookahead = end;
                    while lookahead < bytes.len() && bytes[lookahead].is_ascii_whitespace() {
                        lookahead += 1;
                    }
                    if lookahead < bytes.len() && bytes[lookahead] == b'(' {
                        methods_used.insert(ident);
                    } else {
                        props_used.insert(ident);
                    }
                }
                i = end;
                continue;
            }
        }
        i += 1;
    }

    (props_used, methods_used)
}

#[test]
fn qml_files_exist_and_declare_bound_component_behavior() {
    let main_path = repo_path("qml/Main.qml");
    let step_path = repo_path("qml/StepIndicator.qml");

    let main_qml = std::fs::read_to_string(&main_path).expect("read qml/Main.qml");
    let step_qml = std::fs::read_to_string(&step_path).expect("read qml/StepIndicator.qml");

    assert!(main_qml.contains("pragma ComponentBehavior: Bound"));
    assert!(step_qml.contains("pragma ComponentBehavior: Bound"));

    assert!(main_qml.contains("import dev.dualsense.updater"));
    assert!(main_qml.contains("import org.kde.kirigami as Kirigami"));
    assert!(main_qml.contains("DualSenseBridge {"));
    assert!(main_qml.contains("id: updater"));
    assert!(main_qml.contains("currentStepIndex: updater.stepIndex"));
    assert!(main_qml.contains("currentIndex: updater.stepIndex"));
    assert!(main_qml.contains("firmwareFileDialog.close()"));

    assert!(step_qml.contains("required property int currentStepIndex"));
    assert!(step_qml.contains("required property int index"));
}

#[test]
fn main_qml_only_references_declared_bridge_properties_and_invokables() {
    let bridge_src =
        std::fs::read_to_string(repo_path("src/qt_bridge.rs")).expect("read src/qt_bridge.rs");
    let main_qml = std::fs::read_to_string(repo_path("qml/Main.qml")).expect("read qml/Main.qml");

    let (declared_props, declared_methods) = extract_bridge_members(&bridge_src);
    assert!(
        !declared_props.is_empty(),
        "expected #[qproperty] declarations in src/qt_bridge.rs"
    );
    assert!(
        !declared_methods.is_empty(),
        "expected #[qinvokable] declarations in src/qt_bridge.rs"
    );

    let (used_props, used_methods) = extract_updater_usages(&main_qml);
    assert!(!used_props.is_empty());
    assert!(!used_methods.is_empty());

    let missing_props: Vec<String> = used_props
        .into_iter()
        .filter(|p| !declared_props.contains(p))
        .collect();
    assert!(
        missing_props.is_empty(),
        "Main.qml references undeclared DualSenseBridge properties: {missing_props:?}"
    );

    let missing_methods: Vec<String> = used_methods
        .into_iter()
        .filter(|m| !declared_methods.contains(m))
        .collect();
    assert!(
        missing_methods.is_empty(),
        "Main.qml calls undeclared DualSenseBridge invokables: {missing_methods:?}"
    );
}
