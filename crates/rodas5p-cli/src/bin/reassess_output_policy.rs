//! Read-only assessment of the committed compact export. No ODE run or freeze.
use anyhow::{Context, Result, bail};
use clap::Parser;
use rodas5p_core::sha256_hex;
use rodas5p_fair_ab::{
    GlobalErrorMetric, OutputPolicyMetricKey, OutputSamplingPolicy, assess_policy_measurement,
};
use rodas5p_integrators::{ScientificCaseSpec, ScientificCorpusV2};
use serde_json::{Value, json};
use std::{collections::BTreeSet, fs, io::Write, path::PathBuf};
#[derive(Parser)]
struct Args {
    #[arg(long)]
    input: PathBuf,
    #[arg(long)]
    output: PathBuf,
    /// A priori WRMS budget; omit if none was specified.
    #[arg(long)]
    declared_budget: Option<f64>,
}
fn text<'a>(v: &'a Value, p: &str) -> Result<&'a str> {
    v.pointer(p)
        .and_then(Value::as_str)
        .with_context(|| format!("missing text {p}"))
}
fn number(v: &Value, p: &str) -> Result<f64> {
    let x = v
        .pointer(p)
        .and_then(Value::as_f64)
        .with_context(|| format!("missing number {p}"))?;
    if !x.is_finite() || x < 0. {
        bail!("nonfinite/negative {p}");
    }
    Ok(x)
}
fn count(v: &Value, p: &str) -> Result<u64> {
    v.pointer(p)
        .and_then(Value::as_u64)
        .with_context(|| format!("missing integer {p}"))
}
fn one(rec: &Value, budget: Option<f64>) -> Result<Value> {
    let a = rec.get("artifact").context("artifact missing")?;
    let s = a.get("spec").context("spec missing")?;
    if text(s, "/partition")? != "calibration" {
        bail!("holdout is outside scope");
    }
    let spec: ScientificCaseSpec = serde_json::from_value(s.clone())?;
    let expected = ScientificCorpusV2::calibration_specs()
        .into_iter()
        .find(|x| x.id == spec.id)
        .context("unknown calibration case")?;
    // Compare the scientific input, not provenance prose or JSON presentation.
    // These are the same frozen f64 configuration values, not solver outputs.
    if spec.family != expected.family
        || spec.partition != expected.partition
        || spec.dimension != expected.dimension
        || spec.grid_shape != expected.grid_shape
        || spec.atol != expected.atol
        || spec.rtol != expected.rtol
        || spec.t_span != expected.t_span
        || spec.output_times != expected.output_times
        || spec.mandatory_breakpoints != expected.mandatory_breakpoints
    {
        bail!("calibration specification differs from the declared corpus");
    }
    if number(a, "/config/outer_atol")? != spec.atol
        || number(a, "/config/outer_rtol")? != spec.rtol
    {
        bail!("recorded tolerance differs from case specification");
    }
    let ts = s
        .get("output_times")
        .and_then(Value::as_array)
        .context("grid missing")?;
    if ts.len() < 2 {
        bail!("grid too short");
    }
    let mut prev = f64::NEG_INFINITY;
    for v in ts {
        let t = v.as_f64().context("time not numeric")?;
        if !t.is_finite() || t <= prev {
            bail!("times not finite/increasing");
        }
        prev = t;
    }
    let u = number(a, "/reference_uncertainty_wrms")?;
    let gap = number(a, "/output_policy_discrepancy_wrms")?;
    let mut policies = Vec::new();
    for (name, policy) in [
        ("clipped", OutputSamplingPolicy::Clipped),
        ("dense", OutputSamplingPolicy::Dense),
    ] {
        let arm = a.get(name).context("arm missing")?;
        let good = text(rec, "/status")? == "complete" && text(arm, "/status")? == "success";
        let e = arm
            .pointer("/metrics/max_grid_wrms")
            .and_then(Value::as_f64);
        let m = assess_policy_measurement(policy, good, e, Some(u), budget)?;
        let attempts = count(arm, "/diagnostics/attempts")?;
        let accepted = count(arm, "/diagnostics/accepted_macro_steps")?;
        let rejected = count(arm, "/diagnostics/rejected_macro_steps")?;
        if accepted.checked_add(rejected) != Some(attempts) {
            bail!("attempt accounting does not close");
        }
        if good && count(arm, "/committed_output_count")? != ts.len() as u64 {
            bail!("missing outputs");
        }
        let key = OutputPolicyMetricKey {
            problem_id: text(a, "/reference/problem_id")?.into(),
            output_grid_id: sha256_hex(&serde_json::to_vec(ts)?),
            scale_id: format!(
                "{}:{}",
                text(a, "/reference/wrms_formula_id")?,
                text(a, "/row/binding/campaign/wrms_scale_sha256")?
            ),
            metric: GlobalErrorMetric::MaxGridWrms,
            policy,
        };
        policies.push(json!({"comparison_key":key,"measurement":m,"attempts":attempts,"accepted_steps":accepted,"rejected_steps":rejected,"jvp_vectors":count(arm,"/counters/jvp_vectors")?,"raw_metrics":arm.get("metrics")}));
    }
    let ec = number(&a["clipped"], "/metrics/max_grid_wrms")?;
    let ed = number(&a["dense"], "/metrics/max_grid_wrms")?;
    Ok(
        json!({"case_id":text(s,"/id")?,"family":text(s,"/family")?,"dimension":count(s,"/dimension")?,"rtol":number(s,"/rtol")?,"atol":number(s,"/atol")?,"historical_status":text(a,"/row/status")?,"historical_execution_revision":a.get("code_revision"),"historical_artifact_checksum":a.get("artifact_checksum_sha256"),"assessment_status":"POLICY_RESOLVED_SUMMARY","policies":policies,"direct_trajectory_gap_wrms":gap,"difference_of_scalar_error_norms":(ec-ed).abs(),"raw_state_integrity":"NOT_REVALIDATED_FROM_COMPACT_EXPORT","controller_contamination":"NOT_IDENTIFIABLE_FROM_AGGREGATE_WORK","interpolation_order":"NOT_IDENTIFIABLE_FROM_POLICY_GAP_RATIO","claim_admitted":false}),
    )
}
fn report(v: &Value, budget: Option<f64>, sha: &str) -> Result<Value> {
    if text(v, "/schema")? != "vigilode-scientific-validity-v2-external-reaudit-compact-v1" {
        bail!("unsupported input representation");
    }
    let records = v
        .get("records")
        .and_then(Value::as_array)
        .context("records missing")?;
    if count(v, "/campaign/expected_case_count")? != 54 || records.len() != 54 {
        bail!("requires the complete 54-case compact export");
    }
    let mut ids = BTreeSet::new();
    let mut rows = Vec::new();
    let mut errors = 0;
    for (i, rec) in records.iter().enumerate() {
        let id = rec.pointer("/artifact/spec/id").and_then(Value::as_str);
        let duplicate = id.is_some_and(|x| !ids.insert(x));
        let evaluated = if duplicate {
            Err(anyhow::anyhow!(
                "duplicate case {}",
                id.unwrap_or("unknown")
            ))
        } else {
            one(rec, budget)
        };
        match evaluated {
            Ok(x) => rows.push(x),
            Err(e) => {
                errors += 1;
                rows.push(json!({"record_index":i,"case_id":id,"assessment_status":"ERROR","error":e.to_string(),"claim_admitted":false}));
            }
        }
    }
    Ok(
        json!({"report":"output-policy-resolved-reassessment-20260829","status":if errors==0{"REASSESSED_SUMMARY_ONLY"}else{"COMPLETE_WITH_ERRORS"},"input_sha256":sha,"historical_campaign":v.get("campaign"),"historical_source_campaign":v.get("source_campaign"),"records_retained":rows.len(),"error_records":errors,"declared_budget":budget,"reference_uncertainty_kind":"empirical-estimate-not-a-rigorous-bound","campaign_rerun":false,"historical_artifacts_modified":false,"freeze_created":false,"holdout_opened":false,"claim_admitted":false,"rows":rows}),
    )
}
fn main() -> Result<()> {
    let a = Args::parse();
    let bytes = fs::read(&a.input)?;
    let v: Value = serde_json::from_slice(&bytes)?;
    let r = report(&v, a.declared_budget, &sha256_hex(&bytes))?;
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&a.output)
        .context("output must be a new file")?;
    serde_json::to_writer_pretty(&mut f, &r)?;
    f.write_all(b"\n")?;
    println!("POLICY_REASSESSMENT_WRITTEN {}", a.output.display());
    if r["error_records"].as_u64().unwrap_or(1) > 0 {
        bail!("failed rows preserved in output");
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn empty_complete_is_not_success() {
        assert!(report(&json!({"schema":"vigilode-scientific-validity-v2-external-reaudit-compact-v1","campaign":{"expected_case_count":54},"records":[]}),None,"x").is_err());
    }
    #[test]
    fn bool_is_not_counter() {
        assert!(count(&json!({"n":true}), "/n").is_err());
    }
    #[test]
    fn missing_is_not_zero() {
        assert!(number(&json!({}), "/error").is_err());
    }
    #[test]
    fn all_failed_rows_remain_visible() {
        let v = json!({"schema":"vigilode-scientific-validity-v2-external-reaudit-compact-v1","campaign":{"expected_case_count":54},"records":vec![json!({"status":"failed"});54]});
        let r = report(&v, None, "x").unwrap();
        assert_eq!(r["error_records"], 54);
        assert_eq!(r["rows"].as_array().unwrap().len(), 54);
        assert_eq!(r["claim_admitted"], false);
    }
    fn actual_compact() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../research/scientific_validity_v2_20260829/external_reaudit_bundle/rust/calibration_all_cases_compact.json");
        serde_json::from_slice(&fs::read(path).unwrap()).unwrap()
    }
    #[test]
    fn semantic_spec_drift_is_not_a_complete_calibration_row() {
        let mut v = actual_compact();
        v["records"][0]["artifact"]["spec"]["dimension"] = json!(97);
        let r = report(&v, None, "fixture").unwrap();
        assert_eq!(r["error_records"], 1);
        assert_eq!(r["records_retained"], 54);
    }
    #[test]
    fn duplicate_rows_are_errors_not_silently_dropped() {
        let mut v = actual_compact();
        v["records"][1] = v["records"][0].clone();
        let r = report(&v, None, "fixture").unwrap();
        assert_eq!(r["error_records"], 1);
        assert_eq!(r["records_retained"], 54);
    }
}
