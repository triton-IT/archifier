//! VD-23 : les tokens de la grammaire visuelle existent et le mouvement est
//! neutralisé sous `prefers-reduced-motion`.

use std::fs;
use std::path::Path;

fn tokens() -> String {
    fs::read_to_string(Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets/tokens.css"))
        .unwrap()
}

#[test]
fn tokens_define_every_grammar_family() {
    let css = tokens();
    for family in [
        "--layer-business-bg",
        "--layer-application-bg",
        "--layer-technology-bg",
        "--zone-domain-bg",
        "--zone-group-bg",
        "--boundary-stroke",
        "--port-size",
        "--temporal-active-stroke",
        "--temporal-scheduled-pattern",
        "--temporal-expired-pattern",
        "--draft-stroke",
        "--status-valid-bg",
        "--zoom-aggregate-max",
        "--focus-ring",
        "--motion-fast",
    ] {
        assert!(css.contains(family), "token absent : {family}");
    }
}

#[test]
fn reduced_motion_disables_transitions() {
    let css = tokens();
    assert!(css.contains("prefers-reduced-motion: reduce"));
    assert!(css.contains("--motion-fast: 0ms") || css.contains("--motion-fast: 0"));
}

#[test]
fn layer_tokens_cover_all_profile_layers() {
    let css = tokens();
    for layer in [
        "business",
        "application",
        "technology",
        "physical",
        "motivation",
        "implementation",
        "process",
        "flow",
        "data",
        "annotation",
        "structure",
        "deployment",
        "behavior",
    ] {
        assert!(
            css.contains(&format!("--layer-{layer}-bg")),
            "couche sans token : {layer}"
        );
    }
}
