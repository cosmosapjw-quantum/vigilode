use vigilode::SwitchingEvidence;

#[allow(dead_code)]
fn legacy_name(evidence: SwitchingEvidence) -> &'static str {
    match evidence {
        SwitchingEvidence::Uncompared => "uncompared",
        SwitchingEvidence::ExplicitWorkPressure => "explicit-work-pressure",
        SwitchingEvidence::ImplicitCheaperOnPrefix => "implicit-cheaper-on-prefix",
        SwitchingEvidence::ImplicitNotCheaperOnPrefix => "implicit-not-cheaper-on-prefix",
        SwitchingEvidence::ImplicitPilotIncomplete => "implicit-pilot-incomplete",
        SwitchingEvidence::ExplicitCheaperThanPreviousWindow => "explicit-cheaper-than-previous-window",
        SwitchingEvidence::ExplicitCheaperOnSameWindow => "explicit-cheaper-on-same-window",
        SwitchingEvidence::ImplicitPreferredOnSameWindow => "implicit-preferred-on-same-window",
        SwitchingEvidence::ExplicitReturnNotSupported => "explicit-return-not-supported",
        SwitchingEvidence::ImplicitWindowCompleted => "implicit-window-completed",
    }
}

fn main() {}
