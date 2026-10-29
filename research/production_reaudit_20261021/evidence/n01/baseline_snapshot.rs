use vigilode::{CallbackError, Direction, Jacobian, Options, Problem, Rodas5p};
fn main() {
    let problem = Problem::new(3, |_t, y, dy| {
        dy[0] = -0.04 * y[0] + 1e4 * y[1] * y[2];
        dy[1] = 0.04 * y[0] - 1e4 * y[1] * y[2] - 3e7 * y[1] * y[1];
        dy[2] = 3e7 * y[1] * y[1];
        Ok::<(), CallbackError>(())
    })
    .jacobian(Jacobian::dense(|_t, y, j| {
        j.copy_from_slice(&[-0.04, 1e4*y[2], 1e4*y[1], 0.04, -1e4*y[2]-6e7*y[1], -1e4*y[1], 0.0, 6e7*y[1], 0.0]);
        Ok(())
    }))
    .autonomous()
    .event(|_t, y| y[0]-0.9, Direction::Down);
    let sol=Rodas5p::solve(&problem,(0.0,40.0),&[1.0,0.0,0.0],&Options { dense_output:true,..Default::default() }).unwrap();
    assert!(sol.is_success());
    assert!(!sol.events.is_empty());
    println!("{}",serde_json::to_string_pretty(&sol.stats).unwrap());
}
