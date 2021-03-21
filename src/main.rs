mod line_view;

use line_view::LineView;

use argparse::{ArgumentParser, Store, StoreTrue};
use circular_queue::CircularQueue;
use colored::Colorize;
use regex::Regex;

use std::env;
use std::ffi::OsStr;
use std::fs;
use std::io::{BufRead, BufReader};
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

fn process_file(settings: &Settings, filename: &path::PathBuf) {
    let file = fs::File::open(&filename).unwrap();

    let mut print_after: i32 = 0;
    let mut context: CircularQueue<String> = CircularQueue::with_capacity(settings.context);

    let r = BufReader::new(file).lines();

    for line in r {
        if let Ok(l) = line {
            let lv = LineView::new(&l);

            match match_line(&settings, &lv) {
                MatchType::Match(m) => {
                    for cl in context.iter() {
                        println!("{}", cl);
                    }
                    context.clear();
                    print_line(&lv, &m);
                    print_after = settings.context as i32;
                }
                MatchType::MatchNick => {
                    for cl in context.iter() {
                        println!("{}", cl);
                    }
                    context.clear();
                    println!("{}", &l);
                    print_after = settings.context as i32;
                }
                MatchType::NoMatch => {
                    if print_after > 0 {
                        println!("{}", &l);
                        print_after -= 1;
                        if print_after == 0 {
                            println!("--");
                        }
                    }

                    context.push(l);
                }
                MatchType::Skip => continue,
            }
        }
    }
}

fn process_file_count(settings: &Settings, filename: &path::PathBuf) {
    let file = fs::File::open(&filename).expect("Could not open file");

    let r = BufReader::new(file).lines();

    let mut count = 0;

    for line in r {
        if let Ok(l) = line {
            let lv = LineView::new(&l);

            match match_line(&settings, &lv) {
                MatchType::Match(v) => count += v.len(),
                MatchType::MatchNick => count += 1,
                _ => continue,
            }
        }
    }
    println!(
        "{}{}{}",
        filename.file_name().unwrap().to_str().unwrap().purple(),
        ":".cyan(),
        count
    );
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

fn main() {
    let mut settings = Settings {
        channel: String::from(".*"),
        network: String::from(".*"),
        ..Default::default()
    };

    {
        let mut ap = ArgumentParser::new();

        ap.refer(&mut settings.nickname)
            .add_option(&["-n", "--nickname"], Store, "nickname");
        ap.refer(&mut settings.channel)
            .add_option(&["-c", "--channel"], Store, "channel");
        ap.refer(&mut settings.network)
            .add_option(&["-N", "--network"], Store, "network");
        ap.refer(&mut settings.pattern_string)
            .add_option(&["-e", "--pattern"], Store, "pattern");
        ap.refer(&mut settings.fixed).add_option(
            &["-F", "--fixed"],
            StoreTrue,
            "fixed string search",
        );
        ap.refer(&mut settings.strip_time_stamps).add_option(
            &["-d", "--strip-time-stamps"],
            StoreTrue,
            "strip time stamps",
        );
        ap.refer(&mut settings.strip_joins).add_option(
            &["-j", "--strip-joins"],
            StoreTrue,
            "strip joins/leaves and whatnot",
        );
        ap.refer(&mut settings.context)
            .add_option(&["-C", "--context"], Store, "context lines");
        ap.refer(&mut settings.count)
            .add_option(&["-t", "--count"], StoreTrue, "count");

        ap.parse_args_or_exit();
    }

    validate_settings(&mut settings);

    let files = get_log_files(&settings);

    if !settings.count {
        for f in files {
            process_file(&settings, &f);
        }
    } else {
        for f in files {
            process_file_count(&settings, &f);
        }
    }
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
