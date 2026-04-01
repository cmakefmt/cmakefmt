#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Config {
    pub line_width: usize,
    pub tab_size: usize,
    pub max_empty_lines: usize,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            line_width: 80,
            tab_size: 2,
            max_empty_lines: 1,
        }
    }
}
