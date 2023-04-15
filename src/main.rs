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
    print!("{}", prompt);
    std::io::stdout().flush();
    let mut result = String::new();
    std::io::stdin().read_line(&mut result).unwrap();
    return result;
}

fn main() {
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
        let ice_time: i32 = getline("请输入冰时间（不填直接换行）: ").trim().parse().unwrap_or(0);
        let time: i32 = getline("请输入目标时间: ").trim().parse().unwrap();
        let other = getline("请输入关注的坐标范围（可不填，可填单个坐标，可填用空格分隔的左右边界）: ");
        let mut other: Vec<usize> = other.split_whitespace().map(|x|
            x.parse::<i32>().unwrap() as usize).collect();
        let result = calculate_pos_distribution(&ZOMBIE_DB[&zombie_type], ice_time, time);
        assert!((result.iter().sum::<f64>() - 1.0).abs() < 1e-12);
        let dc = matches!(ZOMBIE_DB[&zombie_type].movement_type, MovementType::DanceCheat);
        let tol = if dc {1e-9} else {1e-12};
        let first = result.iter().position(|&x| x > tol).unwrap();
        let last = 879 - result.iter().rev().position(|&x| x > tol).unwrap();
        if other.len() == 1 {
            println!("{}: {}", other[0], result[other[0]]);
        } else if other.len() == 2 {
            other[0] = max(other[0], if dc {0} else {first});
            other[1] = min(other[1], if dc {879} else {last});
            let mut sum: f64 = result[other[0]..=other[1]].iter().sum();
            if !dc && sum > 1.0 - tol {
                sum = 1.0;
            }
            println!("{}-{}: {}", other[0], other[1], sum);
        } else {
            print!("{}-{}: [", first, last);
            for x in &result[first..last] {
                print!("{:.3e}, ", x);
            }
            println!("{:.3e}]", result[last]);
        }
    }
}
