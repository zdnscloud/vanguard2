use failure::Fail;

#[derive(Debug, Fail)]
pub enum DataSrcError {
    #[fail(display = "cname cann't coexist with rr with same or different type")]
    CNameCoExistsWithOtherRR,

    #[fail(display = "rrset isn't belong current zone")]
    OutOfZone,
}
