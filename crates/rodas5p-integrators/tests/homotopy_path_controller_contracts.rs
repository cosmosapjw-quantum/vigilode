use rodas5p_integrators::{HomotopyPredictor, HomotopyRoundSpec, HomotopyScheduleConfig};

#[test]
fn schedule_contract_accepts_a_monotone_three_round_path() {
    let rounds = vec![
        HomotopyRoundSpec::new(1.0 / 3.0, 0.0, 0, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(2.0 / 3.0, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(1.0, 0.0, 2, 1.0, 1).unwrap(),
    ];
    let schedule = HomotopyScheduleConfig::new(rounds, HomotopyPredictor::Euler).unwrap();
    assert_eq!(schedule.rounds().len(), 3);
    assert_eq!(schedule.predictor(), HomotopyPredictor::Euler);
    assert_eq!(schedule.rounds()[2].q(), 2);
    assert_eq!(schedule.rounds()[2].corrections(), 1);
}

#[test]
fn schedule_contract_rejects_nonmonotone_or_incomplete_lambda_paths() {
    let nonmonotone = vec![
        HomotopyRoundSpec::new(0.7, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(0.6, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(1.0, 0.0, 1, 1.0, 0).unwrap(),
    ];
    assert!(HomotopyScheduleConfig::new(nonmonotone, HomotopyPredictor::Euler).is_err());

    let incomplete = vec![
        HomotopyRoundSpec::new(0.4, 0.0, 1, 1.0, 0).unwrap(),
        HomotopyRoundSpec::new(0.9, 0.0, 1, 1.0, 0).unwrap(),
    ];
    assert!(HomotopyScheduleConfig::new(incomplete, HomotopyPredictor::Euler).is_err());
}

#[test]
fn schedule_contract_rejects_invalid_round_parameters() {
    assert!(HomotopyRoundSpec::new(0.5, -0.1, 1, 1.0, 0).is_err());
    assert!(HomotopyRoundSpec::new(0.5, 0.0, 8, 1.0, 0).is_err());
    assert!(HomotopyRoundSpec::new(0.5, 0.0, 1, 0.0, 0).is_err());
    assert!(HomotopyRoundSpec::new(0.5, 0.0, 1, 1.0, 3).is_err());
    assert!(HomotopyRoundSpec::new(f64::NAN, 0.0, 1, 1.0, 0).is_err());
}
