# vigilode

One problem type, one options type, one solver and one result over the
protected RODAS5P drivers. See the crate documentation (`cargo doc -p vigilode
--open`) for routing and refusals.

## Examples

Each example asserts one check and exits 0:

```sh
cargo run -p vigilode --example robertson     # analytic dense Jacobian
cargo run -p vigilode --example dense_output  # Prothero-Robinson, Solution::at
cargo run -p vigilode --example dae_mass      # index-1 DAE, mass diag(1, 1, 0)
cargo run -p vigilode --example banded_fd     # 1D Brusselator, banded(2, 2)
```
