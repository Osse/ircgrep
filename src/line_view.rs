pub struct LineView<'a> {
    line: &'a str,
    first_tab: usize,
    second_tab: usize,
}

impl LineView<'_> {
    pub fn message(&self) -> &str {
        &self.line[self.second_tab + 1..]
    }

    pub fn nick(&self) -> &str {
        &self.line[self.first_tab + 1..self.second_tab]
    }

    pub fn timestamp(&self) -> &str {
        &self.line[0..self.first_tab]
    }

    pub fn is_join(&self) -> bool {
        let nick = self.nick();
        nick == "<--" || nick == "--" || nick == "-->"
    }

    pub fn new<'a>(line: &'a str) -> LineView {
        let f = line.find('\t').unwrap();
        let s = line.get(f + 1..).unwrap().find('\t').unwrap() + f + 1;

        LineView {
            line: line,
            first_tab: f,
            second_tab: s,
        }
    }
}
