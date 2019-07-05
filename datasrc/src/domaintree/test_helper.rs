use r53::Name;
pub fn name_from_string(s: &str) -> Name {
    Name::new(s).unwrap()
}
