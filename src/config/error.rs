use failure::Fail;
use serde_yaml;
use std::{io, net::IpAddr};

#[derive(Debug, Fail)]
pub enum ConfigError {
    #[fail(display = "IO error: {}", _0)]
    IoError(#[fail(cause)] io::Error),

    #[fail(display = "yaml format error: {}", _0)]
    YamlError(#[fail(cause)] serde_yaml::Error),
}

impl From<io::Error> for ConfigError {
    fn from(e: io::Error) -> Self {
        ConfigError::IoError(e)
    }
}

impl From<serde_yaml::Error> for ConfigError {
    fn from(e: serde_yaml::Error) -> Self {
        ConfigError::YamlError(e)
    }
}
