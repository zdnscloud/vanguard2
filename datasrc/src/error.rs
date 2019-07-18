use failure::Fail;

#[derive(Debug, Fail)]
pub enum DataSrcError {
    #[fail(display = "cname cann't coexist with rr with same or different type")]
    CNameCoExistsWithOtherRR,

    #[fail(display = "rrset isn't belong current zone")]
    OutOfZone,

    #[fail(display = "rrset with type {} doesn't exist", _0)]
    RRsetNotFound(String),

    #[fail(display = "rdata {} doesn't exist", _0)]
    RdataNotFound(String),

    #[fail(display = "name {} doesn't exist", _0)]
    NameNotFound(String),
}
