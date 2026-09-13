//! What Scan and a drift reading answer with, split from [`super`] at the 900-line refusal.

/// A root and a member of different strengths, the member marked missing a
/// name, so a route test cannot pass against a constant.
pub fn repository_scan() -> ipc::RepositoryScan {
    let at = |dir: &str, evidence| ipc::ScannedWorkspace {
        dir: dir.to_string(),
        declared_by: Vec::new(),
        evidence,
        manifests: Vec::new(),
        lockfiles: Vec::new(),
        runnables: Vec::new(),
        tools: Vec::new(),
        services: Vec::new(),
        ports: Vec::new(),
        missing: Vec::new(),
        not_read: Vec::new(),
    };
    let mut member = at("apps/web", ipc::EvidenceStrength::Thin);
    member.missing.push(ipc::MissingName {
        name: "lint".to_string(),
        declared_in: vec!["apps/admin".to_string()],
    });
    ipc::RepositoryScan {
        checkout: "/repo".to_string(),
        workspaces: vec![at(".", ipc::EvidenceStrength::Strong), member],
        ci_commands: Vec::new(),
        not_read: vec![ipc::NotRead {
            file: ".ci/pipeline.yml".to_string(),
            why: "YAML under a hidden directory, which no part of Scan reads".to_string(),
        }],
    }
}

/// One drifted line and one whole one, because a list of either alone would
/// let a route test pass while the verdict was constant.
///
/// The `current` row carries `checked: 0` on purpose: it is the ordinary case
/// — `cargo nextest run --workspace` names no path in any repository — and a
/// fake whose clean row was checked against something would hide the field a
/// surface needs to say what its clean list is about.
pub fn manifest_drift() -> ipc::ManifestDrift {
    ipc::ManifestDrift {
        path: "armada.yml".to_string(),
        checkout: "/repo".to_string(),
        declarations: vec![
            ipc::Declaration {
                section: "checks".to_string(),
                name: "test".to_string(),
                key: "run".to_string(),
                run: "cargo nextest run --workspace".to_string(),
                drift: ipc::Drift::Current { checked: 0 },
                unfollowed: vec![ipc::Unfollowed {
                    word: "nextest".to_string(),
                    why: "not an alias this repository declares, so an external cargo \
                          subcommand installed on the machine"
                        .to_string(),
                }],
            },
            ipc::Declaration {
                section: "checks".to_string(),
                name: "lint".to_string(),
                key: "run".to_string(),
                run: "bash scripts/lint.sh".to_string(),
                drift: ipc::Drift::Gone {
                    missing: vec!["scripts/lint.sh".to_string()],
                },
                // On the `gone` row too: what was not followed is a fact about
                // the line whatever the verdict.
                unfollowed: vec![ipc::Unfollowed {
                    word: "bash".to_string(),
                    why: "not a tool this read follows".to_string(),
                }],
            },
        ],
    }
}
