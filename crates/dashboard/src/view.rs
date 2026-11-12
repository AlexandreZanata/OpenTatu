//! View data shared by every page: the Rust side of the UI components in
//! `templates/components/ui.html`. Owner-facing labels are pt-BR.

use opentatu_core::Severity;

/// Data every page passes to `base.html`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Layout {
    /// OpenTatu version shown in the footer.
    pub version: &'static str,
}

impl Default for Layout {
    fn default() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION"),
        }
    }
}

/// Input of the `ui::badge` component: a pt-BR label and a CSS modifier
/// (`c-badge--<modifier>`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Badge {
    /// Text shown to the owner (pt-BR).
    pub label: &'static str,
    /// CSS modifier, e.g. `critical`.
    pub modifier: &'static str,
}

/// Badge of a severity. The only mapping of severities to labels and colors.
pub fn severity_badge(severity: Severity) -> Badge {
    let (label, modifier) = match severity {
        Severity::Critical => ("Crítica", "critical"),
        Severity::High => ("Alta", "high"),
        Severity::Medium => ("Média", "medium"),
        Severity::Low => ("Baixa", "low"),
        Severity::Info => ("Informativa", "info"),
    };
    Badge { label, modifier }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_severity_has_a_distinct_badge() {
        let modifiers: std::collections::BTreeSet<_> = Severity::ALL
            .iter()
            .map(|s| severity_badge(*s).modifier)
            .collect();
        assert_eq!(modifiers.len(), Severity::ALL.len());
    }
}
