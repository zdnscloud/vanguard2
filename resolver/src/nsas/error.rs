use failure::Fail;

#[derive(Debug, Fail)]
pub enum NSASError {
    #[fail(display = "ns response isn't valid: {}", _0)]
    InvalidNSResponse(String),
}
