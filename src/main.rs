mod line_view;

use line_view::LineView;

use circular_queue::CircularQueue;

#[macro_use]
extern crate clap;

use colored::Colorize;
use regex::Regex;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{stdout, BufRead, BufReader, Write};
use std::option::Option;
use std::path;

#[derive(Default)]
struct Settings {
    nickname: String,
    channel: String,
    network: String,
    pattern_string: String,
    pattern: Option<Regex>,
    context: usize,
    strip_joins: bool,
    strip_time_stamps: bool,
    count: bool,
    fixed: bool,
}

#[derive(Debug, PartialEq)]
enum MatchType {
    Match(Vec<(usize, usize)>),
    MatchNick,
    NoMatch,
    Skip,
}

fn match_line(settings: &Settings, lv: &LineView) -> MatchType {
    if settings.strip_joins && lv.is_join() {
        return MatchType::Skip;
    }

    let nick = lv.nick();

    if !settings.nickname.is_empty() && settings.nickname != nick {
        return MatchType::NoMatch;
    }

    if settings.pattern_string.is_empty() {
        return MatchType::MatchNick;
    }

    let mut v = Vec::<(usize, usize)>::new();

    if !settings.fixed {
        for m in settings
            .pattern
            .as_ref()
            .unwrap()
            .captures_iter(lv.message())
        {
            let c = m.get(0).unwrap();
            v.push((c.start(), c.end()));
        }
    } else {
        for (pos, m) in lv.message().match_indices(&settings.pattern_string) {
            v.push((pos, pos + m.len()));
        }
    }

    if !v.is_empty() {
        MatchType::Match(v)
    } else {
        MatchType::NoMatch
    }
}

fn print_line(lv: &LineView, matches: &[(usize, usize)]) {
    print!("{}\t{}\t", lv.timestamp(), lv.nick());

    let msg = lv.message();

    for p in matches {
        print!("{}", msg.get(0..p.0).unwrap());
        print!("{}", msg.get(p.0..p.1).unwrap().red().bold());
    }

    if let Some(last) = msg.get(matches.last().unwrap().1..) {
        println!("{}", last);
    }
}

fn open_file(filename: &path::PathBuf) -> std::io::Lines<BufReader<std::fs::File>> {
    let file = fs::File::open(&filename).expect("Could not open file");

    BufReader::new(file).lines()
}

fn process_file(
    settings: &Settings,
    filename: &path::PathBuf,
    mut writer: impl Write,
) -> std::io::Result<()> {
    let mut print_after: i32 = 0;
    let mut context = CircularQueue::with_capacity(settings.context);

    for line in open_file(&filename) {
        if let Ok(l) = line {
            let lv = LineView::new(&l);

            match match_line(&settings, &lv) {
                MatchType::Match(m) => {
                    for cl in context.iter() {
                        writeln!(writer, "{}", cl)?;
                    }
                    context.clear();
                    print_line(&lv, &m);
                    print_after = settings.context as i32;
                }
                MatchType::MatchNick => {
                    for cl in context.iter() {
                        writeln!(writer, "{}", cl)?;
                    }
                    context.clear();
                    writeln!(writer, "{}", &l)?;
                    print_after = settings.context as i32;
                }
                MatchType::NoMatch => {
                    if print_after > 0 {
                        writeln!(writer, "{}", &l)?;
                        print_after -= 1;
                        if print_after == 0 {
                            writeln!(writer, "--")?;
                        }
                    }

                    context.push(l);
                }
                MatchType::Skip => continue,
            }
        }
    }

    Ok(())
}

fn process_file_count(
    settings: &Settings,
    filename: &path::PathBuf,
    mut writer: impl Write,
) -> std::io::Result<()> {
    let mut count = 0;

    for line in open_file(&filename) {
        if let Ok(l) = line {
            let lv = LineView::new(&l);

            match match_line(&settings, &lv) {
                MatchType::Match(v) => count += v.len(),
                MatchType::MatchNick => count += 1,
                _ => continue,
            }
        }
    }
    writeln!(
        writer,
        "{}{}{}",
        filename.file_name().unwrap().to_str().unwrap().purple(),
        ":".cyan(),
        count
    )?;

    Ok(())
}

fn get_log_files(settings: &Settings) -> Vec<path::PathBuf> {
    let logdir = env::var("HOME").expect("HOME not set??") + "/.weechat/logs";
    let logpath = path::Path::new(&logdir);

    let file_pattern = format!(
        "^irc\\.{}\\.#*{}\\.weechatlog$",
        settings.network, settings.channel
    );
    let file_pattern = Regex::new(&file_pattern).expect("Invalid regex");

    let mut logfiles = logpath
        .read_dir()
        .expect("Invalid directory")
        .into_iter()
        .map(|e| e.unwrap().path())
        .filter(|p| {
            p.extension() == Some(&OsStr::new("weechatlog"))
                && file_pattern.is_match(p.file_name().unwrap().to_str().unwrap())
        })
        .collect::<Vec<path::PathBuf>>();

    logfiles.sort();

    logfiles
}

fn validate_settings(settings: &mut Settings) {
    if settings.count
        && (settings.strip_joins || settings.strip_time_stamps || settings.context > 0)
    {
        eprintln!("Can't combine --count with options affecting output\n");
        std::process::exit(1);
    }

    if settings.nickname.is_empty() && settings.pattern_string.is_empty() {
        eprintln!("Must give either --pattern or --nickname\n");
        std::process::exit(1);
    }

    if !settings.fixed {
        settings.pattern = Some(Regex::new(&settings.pattern_string).expect("Invalid regex"));
    }
}

fn main() -> Result<(), std::io::Error> {
    let mut settings = Settings {
        channel: String::from(".*"),
        network: String::from(".*"),
        ..Default::default()
    };

    let matches = clap_app!(ircgrep =>
        (version: "0.1.0")
        (author: "Øystein Walle <oystwa@gmail.com>")
        (@arg NICKNAME: -n --nickname +takes_value "nickname")
        (@arg CHANNEL:  -c --channel  +takes_value "channel")
        (@arg PATTERN:  -e --pattern  +takes_value "nickname")
        (@arg NETWORK:  -N --network  +takes_value "network")
        (@arg FIXED:    -f --fixed                 "fixed string search")
        (@arg STRIP_TS: -d --("strip-timestamps")  "strip time stamps")
        (@arg STRIP_J:  -j --("strip-joins")       "strip joins/leaves and whatnot")
        (@arg CONTEXT:  -C --context  +takes_value "context lines")
        (@arg COUNT:    -t --count                 "count")
    )
    .get_matches();

    if let Some(n) = matches.value_of("NICKNAME") {
        settings.nickname = n.to_string();
    }
    if let Some(c) = matches.value_of("CHANNEL") {
        settings.channel = c.to_string();
    }
    if let Some(p) = matches.value_of("PATTERN") {
        settings.pattern_string = p.to_string();
    }
    if let Some(n) = matches.value_of("NETWORK") {
        settings.network = n.to_string();
    }
    settings.fixed = matches.is_present("FIXED");
    settings.strip_time_stamps = matches.is_present("STRIP_TS");
    settings.strip_joins = matches.is_present("STRIP_J");
    settings.context = match matches.value_of("CONTEXT") {
        Some(c) => c.parse::<usize>().expect("a number"),
        None => 0,
    };
    settings.count = matches.is_present("COUNT");

    validate_settings(&mut settings);

    let files = get_log_files(&settings);

    if !settings.count {
        for f in files {
            process_file(&settings, &f, &mut stdout())?;
        }
    } else {
        for f in files {
            process_file_count(&settings, &f, &mut stdout())?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_match_line() {
        let mut settings = Settings::default();
        settings.nickname = String::from("osse");
        settings.fixed = true;
        settings.pattern_string = String::from("diagnosing");

        let line = "2020-06-22 11:18:46	osse	check-ignore is for diagnosing .gitignore issues. it doesn't really have an effect on the repo";
        let lv = LineView::new(&line);

        let m = match_line(&settings, &lv);

        let v = vec![(20, 30)];
        assert_eq!(m, MatchType::Match(v));

        settings.nickname = String::from("foo");

        let m = match_line(&settings, &lv);

        assert_eq!(m, MatchType::NoMatch);
    }

    #[test]
    fn test_match_line_many_matches() {
        let mut settings = Settings::default();
        settings.nickname = String::from("osse");
        settings.fixed = true;
        settings.pattern_string = String::from("re");

        let line = "2020-06-22 11:18:46	osse	check-ignore is for diagnosing .gitignore issues. it doesn't really have an effect on the repo";
        let lv = LineView::new(&line);

        let m = match_line(&settings, &lv);

        let v = vec![(10, 12), (39, 41), (61, 63), (90, 92)];
        assert_eq!(m, MatchType::Match(v));
    }
}
