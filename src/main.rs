#![warn(clippy::pedantic)]
#![allow(clippy::needless_return, clippy::redundant_field_names, clippy::cast_possible_truncation,
         clippy::cast_precision_loss, clippy::cast_sign_loss, clippy::wildcard_imports)]

mod common;
mod calculate_pos_distribution;
mod parse_data;

use common::*;
use calculate_pos_distribution::*;
use std::io::Write;

lazy_static::lazy_static! {
    static ref ZOMBIE_DB: HashMap<ZombieType, ZombieData> =
        parse_data::get_zombie_db(include_bytes!("../assets/data.csv"));
}

fn getline(prompt: &str) -> String {
    print!("{prompt}");
    std::io::stdout().flush().unwrap();
    let mut result = String::new();
    std::io::stdin().read_line(&mut result).unwrap();
    return result;
}

fn main() {
    rayon::ThreadPoolBuilder::new().stack_size(16 << 20).build_global().unwrap();
    loop {
        let zombie_type = getline("请输入僵尸类型: ");
        let zombie_type = zombie_type.trim();
        if zombie_type == "exit" {
            break;
        }
        let zombie_type = ZombieType::from_str(zombie_type);
        if zombie_type.is_err() {
            println!("请确认僵尸类型是否拼写正确");
            continue;
        }
        let zombie_type = zombie_type.unwrap();
        let ice_time: i64 = getline("请输入冰时间（不填直接换行）: ").trim().parse().unwrap_or(0);
        let time: i64 = getline("请输入目标时间: ").trim().parse().unwrap();
        let other = getline("请输入关注的坐标范围（可不填，可填单个坐标，可填用空格分隔的左右边界）: ");
        let other: Vec<usize> = other.split_whitespace().map(|x| x.parse::<usize>().unwrap()).collect();
        let d = calculate_pos_distribution(&ZOMBIE_DB[&zombie_type], ice_time, time);
        let prob_sum: f64 = d.dist.iter().sum();
        assert!((prob_sum - 1.0).abs() < 1e-12, "prob_sum = {prob_sum}");
        let dc = matches!(ZOMBIE_DB[&zombie_type].movement_type, MovementType::DanceCheat);
        if other.len() == 1 {
            println!("{}: {}", other[0], d.dist[other[0]]);
        } else if other.len() == 2 {
            let mut sum: f64 = d.dist[other[0]..=other[1]].iter().sum();
            if sum > 1.0 - (if dc {1e-15} else {1e-12}) {
                sum = 1.0;
            }
            println!("{}-{}: {}", other[0], other[1], sum);
        } else {
            let tol = if dc {1e-9} else {1e-12};
            let first = d.dist.iter().position(|&x| x > tol).unwrap();
            let last = 879 - d.dist.iter().rev().position(|&x| x > tol).unwrap();
            assert!((d.max as usize) - (d.min as usize) == last - first);
            let pos_min = (d.min * 1000.0).floor() / 1000.0;
            let pos_max = (d.max * 1000.0).ceil() / 1000.0;
            print!("{pos_min:.03}-{pos_max:.03}: [");
            for x in &d.dist[first..last] {
                print!("{x:.3e}, ");
            }
            println!("{:.3e}]", d.dist[last]);
        }
    }
}
