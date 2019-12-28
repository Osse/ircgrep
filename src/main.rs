mod line_view;

use line_view::LineView;

use argparse::{ArgumentParser, Store, StoreTrue};
use colored::Colorize;
use regex::Regex;
use circular_queue::CircularQueue;

use std::env;
use std::fs;
use std::io::{BufRead, BufReader};
use std::option::Option;
use std::option::Option::{None, Some};
use std::path;
use std::vec;

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

enum MatchStatus {
    Match(vec::Vec<(usize, usize)>),
    MatchLine,
    NoMatch,
    Skip,
}

fn match_line(settings: &Settings, lv: &LineView) -> MatchStatus {
    let mut nick = lv.nick();

    if settings.strip_joins {
        if nick == "<--" || nick == "--" || nick == "-->" {
            return MatchStatus::Skip;
        }
    }

    if nick.starts_with('@') || nick.starts_with('+') {
        nick = nick.get(1..).unwrap();
    }

    if !settings.nickname.is_empty() && settings.nickname != nick {
        return MatchStatus::NoMatch;
    }

    let mut v = vec::Vec::<(usize, usize)>::new();

    if settings.pattern_string.is_empty() {
        return MatchStatus::MatchLine
    }

    for m in settings
        .pattern
        .as_ref()
        .unwrap()
        .captures_iter(lv.message())
    {
        let kek = m.get(0).unwrap();
        v.push((kek.start(), kek.end()));
    }

    if v.len() > 0 {
        MatchStatus::Match(v)
    } else {
        MatchStatus::NoMatch
    }
}

fn print_line(lv: &LineView, matches: &vec::Vec<(usize, usize)>) {
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
                MatchStatus::Match(m) => {
                    if print_after < 0 {
                        println!("{}", "--");
                    }

                    for cl in context.iter() {
                        println!("{}", cl);
                    }
                    context.clear();
                    print_line(&lv, &m);
                    print_after = settings.context as i32;
                },
                MatchStatus::MatchLine => { if print_after < 0 { println!("{}", "--"); }; println!("{}", &l) },
                MatchStatus::NoMatch => {
                    if print_after > 0 {
                        println!("{}", l);
                    }
                    print_after -= 1;
                    context.push(l);
                },
                MatchStatus::Skip => continue
            }
        }
    }
}

fn process_file_count(settings: &Settings, filename: &path::PathBuf) {
    let file = fs::File::open(&filename).unwrap();

    let r = BufReader::new(file).lines();

    let mut count = 0;

    for line in r {
        if let Ok(l) = line {
            let lv = LineView::new(&l);

            match match_line(&settings, &lv) {
                MatchStatus::Match(v) => count += v.len(),
                MatchStatus::MatchLine => count += 1,
                MatchStatus::NoMatch => continue,
                MatchStatus::Skip => continue,
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

fn get_log_files(settings: &Settings) -> vec::Vec<path::PathBuf> {
    let home = env::var("HOME").unwrap();
    let logdir = home + "/.weechat/logs";
    let logpath = path::Path::new(&logdir);

    let mut logfiles = vec::Vec::<path::PathBuf>::new();

    let file_pattern = format!(
        "^irc\\.{}\\.#*{}\\.weechatlog$",
        settings.network, settings.channel
    );
    let file_pattern = Regex::new(&file_pattern).unwrap();

    for entry in fs::read_dir(logpath).unwrap() {
        let path = entry.unwrap().path();

        match path.extension() {
            None => continue,
            Some(ext) => {
                if ext != "weechatlog" {
                    continue;
                }
            }
        };

        if file_pattern.is_match(path.file_name().unwrap().to_str().unwrap()) {
            logfiles.push(path);
        }
    }

    logfiles
}

fn validate_settings(settings: &mut Settings) {
    if settings.count
        && (settings.strip_joins || settings.strip_time_stamps || settings.context > 0)
    {
        eprintln!(
            "{}\n",
            "Can't combine --count with options affecting output"
        );
        std::process::exit(1);
    }

    if settings.nickname.is_empty() && settings.pattern_string.is_empty() {
        eprintln!("{}\n", "Must give either --pattern or --nickname");
        std::process::exit(1);
    }

    if !settings.fixed {
        settings.pattern = Some(Regex::new(&settings.pattern_string).unwrap());
    }
}

fn main() {
    let mut settings = Settings {
        nickname: String::new(),
        channel: String::from(".*"),
        network: String::from(".*"),
        pattern_string: String::new(),
        pattern: None,
        context: 0,
        strip_joins: false,
        strip_time_stamps: false,
        count: false,
        fixed: false,
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
        ap.refer(&mut settings.fixed)
            .add_option(&["-F", "--fixed"], Store, "fixed string search");
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

    if settings.count {
        for f in get_log_files(&settings) {
            process_file(&settings, &f);
        }
    } else {
        for f in get_log_files(&settings) {
            process_file_count(&settings, &f);
        }
    }
}
