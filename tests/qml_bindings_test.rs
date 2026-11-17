// SPDX-FileCopyrightText: 2026 Azorimor <azorimor@gmail.com>
//
// SPDX-License-Identifier: GPL-3.0-or-later

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
    let licenses_path = repo_path("qml/LicensesWindow.qml");
    let about_path = repo_path("qml/AboutWindow.qml");
    let flash_report_path = repo_path("qml/FlashReportWindow.qml");

    let main_qml = std::fs::read_to_string(&main_path).expect("read qml/Main.qml");
    let step_qml = std::fs::read_to_string(&step_path).expect("read qml/StepIndicator.qml");
    let licenses_qml =
        std::fs::read_to_string(&licenses_path).expect("read qml/LicensesWindow.qml");
    let about_qml = std::fs::read_to_string(&about_path).expect("read qml/AboutWindow.qml");
    let flash_report_qml =
        std::fs::read_to_string(&flash_report_path).expect("read qml/FlashReportWindow.qml");

    assert!(main_qml.contains("pragma ComponentBehavior: Bound"));
    assert!(step_qml.contains("pragma ComponentBehavior: Bound"));
    assert!(licenses_qml.contains("pragma ComponentBehavior: Bound"));
    assert!(about_qml.contains("pragma ComponentBehavior: Bound"));
    assert!(flash_report_qml.contains("pragma ComponentBehavior: Bound"));

    assert!(main_qml.contains("import dev.dualsense.updater"));
    assert!(main_qml.contains("import org.kde.kirigami as Kirigami"));
    assert!(main_qml.contains("DualSenseBridge {"));
    assert!(main_qml.contains("id: updater"));
    assert!(main_qml.contains("LicensesWindow {"));
    assert!(main_qml.contains("licensesText: updater.licensesText"));
    assert!(main_qml.contains("AboutWindow {"));
    assert!(main_qml.contains("appVersion: updater.appVersion"));
    assert!(main_qml.contains("githubUrl: updater.githubUrl"));
    assert!(main_qml.contains("githubPagesUrl: updater.githubPagesUrl"));
    assert!(main_qml.contains("FlashReportWindow {"));
    assert!(main_qml.contains("reportMarkdown: updater.flashReportMarkdown"));
    assert!(main_qml.contains("menuBar: QQC2.MenuBar"));
    assert!(main_qml.contains("text: \"Licenses\""));
    assert!(main_qml.contains("text: \"About Unofficial DualSense Firmware Updater\""));
    assert!(main_qml.contains("text: \"Flash Run Infos\""));
    assert!(main_qml.contains("text: \"Open GitHub Issue\""));
    assert!(main_qml.contains("currentStepIndex: updater.stepIndex"));
    assert!(main_qml.contains("currentIndex: updater.stepIndex"));
    assert!(main_qml.contains("firmwareFileDialog.close()"));

    assert!(step_qml.contains("required property int currentStepIndex"));
    assert!(step_qml.contains("required property int index"));

    assert!(licenses_qml.contains("required property string licensesText"));
    assert!(licenses_qml.contains("title: \"Licenses & Third-Party Notices\""));
    assert!(licenses_qml.contains("QQC2.TextArea"));
    assert!(licenses_qml.contains("readOnly: true"));
    assert!(licenses_qml.contains("text: \"Close\""));

    assert!(about_qml.contains("required property string appVersion"));
    assert!(about_qml.contains("required property string githubUrl"));
    assert!(about_qml.contains("required property string githubPagesUrl"));
    assert!(about_qml.contains("title: \"About Unofficial DualSense Firmware Updater\""));
    assert!(about_qml.contains(dualsense_updater::app_icon::EMBEDDED_APP_ICON_URL));

    assert!(flash_report_qml.contains("required property string reportTitle"));
    assert!(flash_report_qml.contains("required property string reportMarkdown"));
    assert!(flash_report_qml.contains("required property string issueUrl"));
    assert!(flash_report_qml.contains("title: \"Flash Run Infos\""));
    assert!(flash_report_qml.contains("text: \"Copy Report\""));
    assert!(flash_report_qml.contains("text: \"Open GitHub Issue\""));
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

#[test]
fn step_indicator_avoids_basictheme_positive_green_on_green_contrast_bug() {
    let step_qml = std::fs::read_to_string(repo_path("qml/StepIndicator.qml"))
        .expect("read qml/StepIndicator.qml");

    assert!(
        step_qml.contains("Kirigami.ColorUtils.tintWithAlpha"),
        "StepIndicator.qml must compute a tinted fallback when Kirigami BasicTheme sets positiveBackgroundColor == positiveTextColor"
    );
    assert!(
        step_qml.contains("Kirigami.Theme.positiveBackgroundColor")
            && step_qml.contains("Qt.colorEqual"),
        "StepIndicator.qml should check whether positiveBackgroundColor equals positiveTextColor"
    );

    let label_section = step_qml
        .split("QQC2.Label {")
        .nth(1)
        .expect("StepIndicator.qml should contain a QQC2.Label");
    assert!(
        !label_section.contains("Kirigami.Theme.positiveTextColor"),
        "StepIndicator.qml label must not pair positiveTextColor foreground against positiveBackgroundColor / green tint"
    );
    assert!(
        label_section.contains("Kirigami.Theme.highlightedTextColor")
            && label_section.contains("Kirigami.Theme.textColor"),
        "StepIndicator.qml label must use highlightedTextColor for current step and textColor for done/pending steps"
    );
    assert!(
        step_qml.contains("border.color: stepDelegate.isCurrent")
            && step_qml.contains("Kirigami.Theme.positiveTextColor"),
        "StepIndicator.qml should keep positiveTextColor on the done step border"
    );
}

#[test]
fn main_qml_status_banner_guards_against_basictheme_white_active_text() {
    let main_qml = std::fs::read_to_string(repo_path("qml/Main.qml")).expect("read qml/Main.qml");
    let banner_section = main_qml
        .split("id: statusBanner")
        .nth(1)
        .and_then(|rest| rest.split("Kirigami.AbstractCard").next())
        .expect("Main.qml should define statusBanner InlineMessage");

    assert!(
        banner_section.contains("Kirigami.Theme.activeTextColor")
            && banner_section.contains("highlightedTextColor")
            && banner_section.contains("highlightColor"),
        "statusBanner must guard activeTextColor against Fusion BasicTheme's white highlightedTextColor"
    );
}
