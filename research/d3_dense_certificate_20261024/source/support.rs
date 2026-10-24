#[derive(Debug,Clone,PartialEq)]
pub enum CoreError { InvalidInput(String), NonFinite(String) }
pub type CoreResult<T> = Result<T,CoreError>;
