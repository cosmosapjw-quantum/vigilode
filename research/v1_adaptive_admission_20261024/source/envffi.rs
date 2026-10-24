#[path="integer_gate.rs"]mod integer_gate;
#[path="admission.rs"]mod admission;
#[no_mangle]pub extern "C" fn environment_code()->i32{if admission::environment().is_ok(){0}else{1}}
