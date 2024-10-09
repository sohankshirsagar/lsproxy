use std::process::Child;

pub struct LspClient {
    process: Child,
}

impl LspClient {
    pub fn new(process: Child) -> Self {
        Self { process }
    }
}
