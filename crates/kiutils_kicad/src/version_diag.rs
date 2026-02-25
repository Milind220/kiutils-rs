use crate::diagnostic::{Diagnostic, Severity};
use crate::version::VersionPolicy;

pub(crate) fn collect_version_diagnostics(version: Option<i32>) -> Vec<Diagnostic> {
    let policy = VersionPolicy::default();
    let mut diagnostics = Vec::new();

    if let Some(v) = version {
        if policy.reject_older && !policy.accepts(v) {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                code: "legacy_format",
                message: format!(
                    "version {v} is older than v9 target; parsing in compatibility mode"
                ),
                span: None,
                hint: Some("parser support for pre-v9 token variants is best-effort".to_string()),
            });
        }

        if policy.is_future_for_target(v) {
            diagnostics.push(Diagnostic {
                severity: Severity::Warning,
                code: "future_format",
                message: format!(
                    "version {v} is newer than target {:?}; keeping lossless CST for compatibility",
                    policy.target
                ),
                span: None,
                hint: Some("consider newer parser coverage for this token set".to_string()),
            });
        }
    }

    diagnostics
}
