use crate::config::Config;
use crate::dict::AllowLists;

#[derive(Debug)]
pub struct Ctx<'a> {
    pub config: &'a Config,
    pub allow: &'a AllowLists,
}

impl<'a> Ctx<'a> {
    pub fn new(config: &'a Config, allow: &'a AllowLists) -> Self {
        Self { config, allow }
    }
}
