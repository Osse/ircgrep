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
        let nick = &self.line[self.first_tab + 1..self.second_tab];
        match nick.strip_prefix(&['@', '+'][..]) {
            Some(n) => n,
            None => nick,
        }
    }

    pub fn timestamp(&self) -> &str {
        &self.line[0..self.first_tab]
    }

    pub fn is_join(&self) -> bool {
        let nick = self.nick();
        nick == "<--" || nick == "--" || nick == "-->"
    }

    pub fn new(line: &str) -> LineView {
        let first_tab = line.find('\t').unwrap();
        let second_tab = line.get(first_tab + 1..).unwrap().find('\t').unwrap() + first_tab + 1;

        LineView {
            line,
            first_tab,
            second_tab,
        }
    }
}
