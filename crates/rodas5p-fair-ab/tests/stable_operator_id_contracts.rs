//! A1 defect (production plan 2026-10-20, section 6): `StableDenseOperator::new`
//! sliced `&operator_id[..16]`, which panics (an abort under the release
//! profile's `panic = "abort"`) on an id shorter than 16 bytes or with a
//! non-ASCII character across byte 16. Such ids are a typed error; a valid
//! hex digest keeps its token.

use rodas5p_core::{CoreError, DenseMatrix, LinearOperator};
use rodas5p_fair_ab::StableDenseOperator;

fn matrix() -> DenseMatrix {
    DenseMatrix::new(1, 1, vec![2.0]).unwrap()
}

fn construct(id: &str) -> std::thread::Result<Result<u64, CoreError>> {
    let id = id.to_owned();
    std::panic::catch_unwind(move || StableDenseOperator::new(matrix(), &id).map(|op| op.token()))
}

#[test]
fn short_operator_id_is_a_typed_error_not_a_panic() {
    for id in ["", "0", "0123456789abcde"] {
        let result = construct(id).unwrap_or_else(|_| panic!("id {id:?} panicked"));
        assert!(
            matches!(result, Err(CoreError::InvalidInput(_))),
            "id {id:?}: {result:?}"
        );
    }
}

#[test]
fn non_ascii_operator_id_is_a_typed_error_not_a_panic() {
    // "é" is two bytes, so byte 16 falls inside it.
    for id in [
        "0123456789abcdeé0000",
        "é0123456789abcdef",
        "ééééééééééééééééé",
    ] {
        let result = construct(id).unwrap_or_else(|_| panic!("id {id:?} panicked"));
        assert!(
            matches!(result, Err(CoreError::InvalidInput(_))),
            "id {id:?}: {result:?}"
        );
    }
}

#[test]
fn hex_digest_prefix_sets_the_token() {
    let digest = "00000000000000ff0123456789abcdef0123456789abcdef0123456789abcdef";
    assert_eq!(construct(digest).unwrap().unwrap(), 0xff);
    assert_eq!(
        construct("0123456789abcdef").unwrap().unwrap(),
        0x0123_4567_89ab_cdef
    );
    // A non-ASCII character after byte 16 does not matter.
    assert_eq!(construct("0000000000000001é").unwrap().unwrap(), 1);
}
