use std::cmp::{max, Ordering};
use std::fmt::format;
use std::fs::File;
use std::io::{read_to_string, stdin, Write};
use std::mem::take;
use std::str::FromStr;
use std::time::Instant;

use notcurses::{Input, InputType, KeyMod, MiceEvents, Notcurses, Plane, Position, Received};
use notcurses::Received::Key;
use notcurses::sys::c_api::libc::send;
use notcurses::sys::c_api::NCKEY_BACKSPACE;
use reqwest::*;

fn query(query: String) -> reqwest::Result<reqwest::blocking::Response> {
    reqwest::blocking::get(
        Url::parse_with_params(
            "http://localhost:9000/exp",
            [("query", query)],
        ).unwrap()
    )
}

fn main() {
    let mut nc = Notcurses::new_cli().unwrap();
    nc.mice_enable(MiceEvents::All).unwrap();
    let mut plane = Plane::new(&mut nc).unwrap();
    nc.refresh().unwrap();

    let send_button = (plane.size().0 / 2 - 5, plane.size().1 / 2 - 1 - 6, 10, 2);
    let ip_address = (plane.size().0 / 2 - 10, plane.size().1 / 2 - 1 - 10, 20, 2);
    let response = (plane.size().0 / 2 - 40, plane.size().1 / 2 - 3, 80, 6);
    let time = (plane.size().0 / 2 - 10, plane.size().1 / 2 - 1 + 6, 20, 2);
    render(&mut plane, send_button, "Submit!");
    render(&mut plane, response, "");
    render(&mut plane, time, "");
    render(&mut plane, ip_address, "___.___.___.___");

    plane.render().unwrap();

    let mut ip: Vec<String> = vec!["".to_string(), "".to_string(), "".to_string(), "".to_string()];
    let mut ip_pointer = 0;

    loop {
        let event = nc.poll_event().unwrap();
        if event.is_received() {
            if event.itype == InputType::Press {
                if intersects(event.cell.unwrap(), send_button) {
                    submit(&mut plane, ip.iter().map(|l| "0".repeat(3 - l.len()) + l).collect::<Vec<String>>().join(".").as_str(), time, response);
                }
            }
            let mut flag = false;
            match event.received {
                Received::NoInput => {}
                Received::Key(k) => {
                    if k == notcurses::Key::Backspace {
                        match ip[ip_pointer].pop() {
                            None => {ip_pointer = (ip_pointer as i32 - 1_i32).max(0) as usize }
                            Some(_) => {}
                        }
                        flag = true
                    } else if k == notcurses::Key::Enter {
                        submit(&mut plane, ip.iter().map(|l| "0".repeat(3 - l.len()) + l).collect::<Vec<String>>().join(".").as_str(), time, response);
                    }
                }
                Received::Char(f) => {
                    if f.is_ascii_digit() {
                        if ip[ip_pointer].len() < 3 {
                            ip[ip_pointer] += f.to_string().as_str();
                        }
                        if ip[ip_pointer].len() >= 3 { ip_pointer = (ip_pointer + 1).min(3) }
                    } else if f == '.' {
                        ip_pointer = (ip_pointer + 1).min(3)
                    }
                    flag = true
                }
            }
            if flag {
                render(&mut plane, ip_address, ip.iter().map(|l| "_".repeat(3 - l.len()) + l).collect::<Vec<String>>().join(".").as_str());
                plane.render().unwrap();
            }
        }
    }
}

fn submit(pl: &mut Plane, ip: &str, time: (u32, u32, u32, u32), response: (u32, u32, u32, u32)) {
    let now = Instant::now();
    let c = ip_to_int(ip);
    let q = query(format!("SELECT id, country, stateprov, city FROM ips WHERE ip_start <= {} AND ip_end >= {} LIMIT 1", c, c)).unwrap();
    let elapsed = now.elapsed();

    render(pl, response, csvize(q.text().unwrap().as_str()).as_str());
    render(pl, time, format!("sex: {}", elapsed.as_secs_f64()).as_str());

    pl.render().unwrap();
}

fn render(plane: &mut Plane, bounding_box: (u32, u32, u32, u32), content: &str) {
    for x in bounding_box.0..=bounding_box.0 + bounding_box.2 {
        for y in bounding_box.1..=bounding_box.1 + bounding_box.3 {
            if x == bounding_box.0 || x == bounding_box.0 + bounding_box.2 {
                plane.putstr_at_xy(Some(x), Some(y), "|").unwrap();
            } else if y == bounding_box.1 || y == bounding_box.1 + bounding_box.3 {
                plane.putstr_at_xy(Some(x), Some(y), "-").unwrap();
            } else {
                plane.putstr_at_xy(Some(x), Some(y), " ").unwrap();
            }
        }
    }
    for l in content.clone().lines().enumerate() {
        plane.putstr_at_xy(
            Some(bounding_box.0 + bounding_box.2 / 2 - (
                content
                    .lines()
                    .max_by(|&a, &b| if a.len() > b.len() { Ordering::Greater } else { Ordering::Less })
                    .unwrap()
                    .chars()
                    .count() / 2
            ) as u32),
            Some(
                ((bounding_box.1 + bounding_box.3 / 2) as i32 -
                    ((content.lines().count() / 2) as i32 - l.0 as i32)) as u32),
            l.1,
        ).unwrap();
    }
}

fn intersects(cursor: Position, bounding_box: (u32, u32, u32, u32)) -> bool {
    return cursor.0 >= bounding_box.0 as i32 && cursor.0 <= (bounding_box.0 + bounding_box.2) as i32
        && cursor.1 >= bounding_box.1 as i32 && cursor.1 <= (bounding_box.1 + bounding_box.3) as i32;
}

fn csvize(str: &str) -> String {
    let l = str.lines().map(|i| i.split(","));
    let mut vc: Vec<Vec<String>> = vec![];
    let mut s : Vec<String> = vec![];
    for c in 0..l.clone().count() { vc.push(vec![]) }
    for j in l.clone().into_iter().nth(0).unwrap().enumerate() {
        let strs = l.clone().enumerate().map(|i| i.1.into_iter().nth(j.0).unwrap().replace("'", "").replace("\"", "")).collect::<Vec<String>>();
        let max = strs.clone().into_iter().max_by(|a, b| if a.len() > b.len() { Ordering::Greater } else { Ordering::Less }).unwrap().len();
        for s in strs.iter().enumerate() {
            vc[s.0].push(s.1.clone().to_string() + " ".repeat(max - s.1.len()).as_str());
        }
        s.push("-".repeat(max))
    }
    vc.insert(1, s);
    vc.iter().map(|c| c.join(" | ")).collect::<Vec<String>>().join("\n")
}

fn csv() {
    let fl = read_to_string(File::open("/home/pavel/Downloads/DbIp_Edu.sql").unwrap()).unwrap();
    let lines = fl.lines().skip(42).take(698 - 42).collect::<Vec<&str>>().join("").replace("\n", "");
    let x = lines
        .split("),")
        .map(|record| &record[(record.chars().position(|c| c == '(').unwrap() + 1)..record.len()])
        .filter_map(|record| {
            let mut rs = record.split(",");
            // println!("{:?}", rs.clone().into_iter().collect::<Vec<&str>>().join(""));
            let ipv6 = rs.nth(1).unwrap();
            if ipv6 == "1" { return None; }
            Some(format!("{},{},{},{},{},{},{}",
                         rs.nth(0).unwrap(),
                         ipv6,
                         ip_to_int(rs.nth(0).unwrap()),
                         ip_to_int(rs.nth(0).unwrap()),
                         rs.nth(0).unwrap(),
                         rs.nth(0).unwrap(),
                         rs.nth(0).unwrap(),
            ))
        })
        .collect::<Vec<String>>().join("\n")
        ;
    // println!("{}", x);
    File::create("/home/pavel/Downloads/data.csv")
        .unwrap()
        .write_all(x.as_bytes())
        .unwrap();
}

fn ip_to_int(s: &str) -> u32 {
    let a = s.replace("'", "").split(".")
        .map(|e| {
            format!("{:08b}", u8::from_str(e).unwrap())
        }).collect::<Vec<String>>().join("");
    return u32::from_str_radix(a.as_str(), 2).unwrap();
}