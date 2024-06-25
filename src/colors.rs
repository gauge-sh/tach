pub struct BColors;

impl BColors {
    pub const HEADER: &'static str = "\x1b[95m";
    pub const OKBLUE: &'static str = "\x1b[94m";
    pub const OKCYAN: &'static str = "\x1b[96m";
    pub const OKGREEN: &'static str = "\x1b[92m";
    pub const WARNING: &'static str = "\x1b[93m";
    pub const FAIL: &'static str = "\x1b[91m";
    pub const ENDC: &'static str = "\x1b[0m";
    pub const BOLD: &'static str = "\x1b[1m";
    pub const UNDERLINE: &'static str = "\x1b[4m";
}
