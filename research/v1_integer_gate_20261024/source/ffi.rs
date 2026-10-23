#[path="integer_gate.rs"]mod gate;
/// C ABI contract: e points to30 readable u64, s to33 readable u64,
/// out to6 writable u64; no overlap and valid alignment. Null rejected.
/// Test-only ABI; the safe research API is gate::evaluate with fixed arrays.
#[no_mangle]
pub unsafe extern "C" fn gate_bits(e:*const u64,s:*const u64,out:*mut u64)->i32{
 if e.is_null()||s.is_null()||out.is_null(){return 3}
 let e=unsafe{&*(e as *const [u64;30])};let s=unsafe{&*(s as *const [u64;33])};let o=unsafe{&mut*(out as *mut[u64;6])};
 *o=[0;6];match gate::evaluate(e,s){
  Err(_)=>3,
  Ok(c)=>{match c{gate::Certificate::Bound{lo,hi}=>{o[0]=lo as u64;o[1]=(lo>>64)as u64;o[2]=hi as u64;o[3]=(hi>>64)as u64;},gate::Certificate::LargeRow{row}=>{o[4]=1;o[5]=row as u64;}}
   match c.decision(){gate::Decision::Accept=>0,gate::Decision::Reject=>1,gate::Decision::Indeterminate=>2}}
 }
}
