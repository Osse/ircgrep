pub struct LineView<'a> {
    line: &'a str,
    first_tab: usize,
    second_tab: usize,
}

impl<'a> LineView<'a> {
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

    pub fn new(line: &'a str) -> LineView<'a> {
        let first_tab = line.find('\t').unwrap();
        let second_tab = line.get(first_tab + 1..).unwrap().find('\t').unwrap() + first_tab + 1;

        LineView {
            line,
            first_tab,
            second_tab,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn basic_parsing() {
        let line = "2020-06-22 11:18:46	osse	check-ignore is for diagnosing .gitignore issues. it doesn't really have an effect on the repo";
        let lv = LineView::new(&line);

        assert_eq!(lv.timestamp(), "2020-06-22 11:18:46");
        assert_eq!(lv.nick(), "osse");
        assert_eq!(lv.is_join(), false);
        assert_eq!(lv.message(), "check-ignore is for diagnosing .gitignore issues. it doesn't really have an effect on the repo");
    }

    #[test]
    fn basic_parsing2() {
        let line = "2020-06-22 11:40:05	<--	roadie (~user@2a02:8108:ec0:1427:38ed:3aa7:170e:5e4e) has quit (Remote host closed the connection)";
        let lv = LineView::new(&line);

        assert_eq!(lv.timestamp(), "2020-06-22 11:40:05");
        assert_eq!(lv.nick(), "<--");
        assert_eq!(lv.is_join(), true);
        assert_eq!(lv.message(), "roadie (~user@2a02:8108:ec0:1427:38ed:3aa7:170e:5e4e) has quit (Remote host closed the connection)");
    }
}
