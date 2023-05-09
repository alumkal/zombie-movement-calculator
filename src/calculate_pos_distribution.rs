use crate::common::*;
use libm::erfc;
use num_traits::ToPrimitive;
use rayon::prelude::*;

fn calc_time(data: &ZombieData, ice_time: i64, time: i64) -> (i64, i64) {
    let norm_time = ice_time - 1; // 僵尸原速移动的时间
    if ice_time == 0 || ice_time > time || data.chill_immune { (time, 0) }
    else if data.freeze_immune { (norm_time + max(time - norm_time - 1999, 0), min(time - norm_time, 1999)) }
    else { (norm_time + max(time - norm_time - 1999, 0), max(min(time - norm_time, 1999) - 399, 0)) }
}

fn calculate_constant(data: &ZombieData, ice_time: i64, time: i64) -> PosDistribution {
    let speed_min_norm = (data.speed.0 * 16384).round() / 16384;
    let speed_max_norm = (data.speed.1 * 16384).round() / 16384;
    let speed_min_chill = (data.speed.0 * Num::new(2, 5) * 16384).round() / 16384;
    let speed_max_chill = (data.speed.1 * Num::new(2, 5) * 16384).round() / 16384;
    let mut contrib = [0.0; 880];
    let (norm_time, chill_time_max) = calc_time(data, ice_time, time);
    let chill_time_min = if data.freeze_immune {chill_time_max} else {max(chill_time_max - 200, 0)};
    let minimum_chill_multiplier = 201 - (chill_time_max - chill_time_min);
    let spawn_span = data.spawn.1 - data.spawn.0 + 1;
    for chill_time in chill_time_min..=chill_time_max {
        let weight = Num::new(if chill_time == chill_time_min {minimum_chill_multiplier} else {1}, 201) / spawn_span;
        let dx_min = speed_min_norm * norm_time + speed_min_chill * chill_time;
        let dx_max = speed_max_norm * norm_time + speed_max_chill * chill_time;
        let pos_min = data.spawn.1 - dx_max.ceil().to_integer();
        let pos_max = data.spawn.1 - dx_min.ceil().to_integer();
        if pos_min == pos_max {
            contrib[pos_min as usize] += weight.to_f64().unwrap();
        } else {
            let l_ratio = (dx_max - dx_max.floor()) / (dx_max - dx_min);
            let r_ratio = (dx_min.ceil() - dx_min) / (dx_max - dx_min);
            contrib[pos_min as usize] += (weight * l_ratio).to_f64().unwrap();
            contrib[pos_max as usize] += (weight * r_ratio).to_f64().unwrap();
            for i in (pos_min + 1)..pos_max {
                contrib[i as usize] += (weight / (dx_max - dx_min)).to_f64().unwrap();
            }
        }
    }
    let global_dx_min = speed_min_norm * norm_time + speed_min_chill * chill_time_min;
    let global_dx_max = speed_max_norm * norm_time + speed_max_chill * chill_time_max;
    let mut result = PosDistribution {
        dist: [0.0; 880],
        min: (Num::new(data.spawn.0, 1) - global_dx_max).to_f64().unwrap(),
        max: (Num::new(data.spawn.1, 1) - global_dx_min).to_f64().unwrap()
    };
    for i in 0..880 {
        result.dist[i] = contrib[i..min(i + spawn_span as usize, 880)].iter().sum();
    }
    return result;
}

fn prob_between(l: f64, r: f64) -> f64 {
    if l.abs() > r.abs() { prob_between(-r, -l) }
    else { (erfc(l / 2.0_f64.sqrt()) - erfc(r / 2.0_f64.sqrt())) / 2.0 }
}

fn calculate_dancecheat(data: &ZombieData, ice_time: i64, time: i64) -> PosDistribution {
    let k = data.speed.0.to_f64().unwrap();
    let mut contrib = [0.0; 880];
    let (norm_time, chill_time_max) = calc_time(data, ice_time, time);
    let chill_time_min = if data.freeze_immune {chill_time_max} else {max(chill_time_max - 200, 0)};
    let minimum_chill_multiplier = 201 - (chill_time_max - chill_time_min);
    let spawn_span = data.spawn.1 - data.spawn.0 + 1;
    for chill_time in chill_time_min..=chill_time_max {
        let weight = (if chill_time == chill_time_min {minimum_chill_multiplier as f64} else {1.0}) / 201.0
        / spawn_span as f64;
        let norm_time = norm_time as f64;
        let chill_time = chill_time as f64;
        let mean = (data.spawn.1 as f64) - k * (norm_time + chill_time / 2.0);
        let std = k * (49.0 / 2700.0 * (norm_time + chill_time / 4.0)).sqrt();
        let pos_min = (mean - 10.0 * std) as usize;
        let pos_max = (mean + 10.0 * std) as usize;
        for pos in pos_min..=pos_max {
            let l = (pos as f64 - mean) / std;
            let r = ((pos + 1) as f64 - mean) / std;
            contrib[pos] += weight * prob_between(l, r);
        }
    }
    let mut dist = [0.0; 880];
    for i in 0..880 {
        dist[i] = contrib[i..min(i + spawn_span as usize, 880)].iter().sum();
    }
    return PosDistribution {
        dist: dist,
        min: dist.iter().position(|&x| x > 1e-9).unwrap() as f64,
        max: 879.999 - dist.iter().rev().position(|&x| x > 1e-9).unwrap() as f64
    };
}

// 返回分母 <=n 且在 (l, r) 之间的所有分数，外加 l 和 r
fn fraction_between(n: i64, l: Num, r: Num) -> Vec<Num> {
    let mut result = vec![l];
    for i in 1..=n {
        let den_l = l * i;
        let den_r = r * i;
        let den_l = if den_l.is_integer() {den_l + 1} else {den_l.ceil()};
        let den_r = if den_r.is_integer() {den_r - 1} else {den_r.floor()};
        for j in den_l.to_integer()..=den_r.to_integer() {
            if num_integer::gcd(i, j) == 1 {
                result.push(Num::new(j, i));
            }
        }
    }
    result.push(r);
    result.sort_unstable();
    return result;
}

fn calculate_animation(data: &ZombieData, ice_time: i64, time: i64, animation: Option<&Vec<Num>>) -> PosDistribution {
    let animation = animation.unwrap_or(match &data.movement_type {
        MovementType::Animation(x) | MovementType::Dancing(x) => x,
        _ => unreachable!()
    });
    let anim_len = animation.len() as i64;
    let total: Num = animation.iter().sum();
    let speed_scale_factor = Num::new(47, 100) * anim_len / total;
    let dis_scale_factor = Num::new(anim_len + 1, anim_len);
    // 原速 norm_time cs, 减速 [chill_time_max-200, chill_time_max] cs
    let (norm_time, chill_time_max) = calc_time(data, ice_time, time);
    let n = norm_time * 2 + chill_time_max;
    // k 是减速状态下相位的变化率
    let k_min = data.speed.0 * speed_scale_factor / 2;
    let k_max = data.speed.1 * speed_scale_factor / 2;
    // k 在 [k_segments[i], k_segments[i+1]) 范围内变化时 dx 正比于 k
    let k_segments =
        if chill_time_max != 0 { fraction_between(n, k_min, k_max) }
        else { fraction_between(n / 2, k_min * 2, k_max * 2).iter().map(|x| x / 2).collect() };
    // 举例：chill_time = 100 时，实际减速时间取到最小值 0 的概率是 101/201 (冰 500-600cs)，是取到其他数值的 101 倍
    let minimum_chill_multiplier = max(200 - chill_time_max + 1, 1);
    let (contrib, dx_global_min, dx_global_max) = k_segments[0..(k_segments.len() - 1)]
        .par_iter()
        .zip(k_segments[1..(k_segments.len())].par_iter())
        .map(|(l, r)| {
        let mut contrib = [0.0; 880];
        let mut dx_global_min = Num::new(1000, 1);
        let mut dx_global_max = Num::new(0, 1);
        let mut dx_min = Num::new(0, 1);
        let mut dx_max = Num::new(0, 1);
        let mut phase = l * 2;
        for _ in 0..norm_time {
            let shift = animation[phase.to_integer() as usize % animation.len()] * dis_scale_factor * 2;
            dx_min += (shift * l * 16384).round() / 16384;
            dx_max += (shift * r * 16384).round() / 16384;
            phase += l * 2;
        }
        for _ in 0..max(chill_time_max - 200, 0) {
            let shift = animation[phase.to_integer() as usize % animation.len()] * dis_scale_factor;
            dx_min += (shift * l * 16384).round() / 16384;
            dx_max += (shift * r * 16384).round() / 16384;
            phase += l;
        }
        for i in 0..=min(chill_time_max, 200) {
            // 逐个更新不同冻结时间的期望
            dx_global_min = min(dx_global_min, dx_min);
            dx_global_max = max(dx_global_max, dx_max);
            let spawn_span = data.spawn.1 - data.spawn.0 + 1;
            let weight = Num::new(if i == 0 {minimum_chill_multiplier} else {1}, 201) / spawn_span
                * (if k_min == k_max {Num::new(1, 1)} else {(r - l) / (k_max - k_min)}); // avoid 0/0
            let dx_l = dx_min.ceil().to_integer();
            let dx_r = dx_max.ceil().to_integer();
            for dx in dx_l..=dx_r {
                let ratio =
                    if dx_min == dx_max {Num::new(1, 1)}
                    else {
                        let ratio_l = (max(Num::new(dx - 1, 1), dx_min) - dx_min) / (dx_max - dx_min);
                        let ratio_r = (min(Num::new(dx, 1), dx_max) - dx_min) / (dx_max - dx_min);
                        ratio_r - ratio_l
                    };
                contrib[(data.spawn.1 - dx) as usize] += (weight * ratio).to_f64().unwrap();
            }
            let shift = animation[phase.to_integer() as usize % animation.len()] * dis_scale_factor;
            dx_min += (shift * l * 16384).round() / 16384;
            dx_max += (shift * r * 16384).round() / 16384;
            phase += l;
        }
        (contrib, dx_global_min, dx_global_max)
    }).reduce(|| { ([0.0; 880], Num::new(1000, 1), Num::new(0, 1)) },
        |(contrib, dx_global_min, dx_global_max), (contrib_, dx_global_min_, dx_global_max_)| {
        let mut contrib = contrib;
        for i in 0..880 {
            contrib[i] += contrib_[i];
        }
        (contrib, min(dx_global_min, dx_global_min_), max(dx_global_max, dx_global_max_))
    });
    let mut result = PosDistribution {
        dist: [0.0; 880],
        min: (Num::new(data.spawn.0, 1) - dx_global_max).to_f64().unwrap(),
        max: (Num::new(data.spawn.1, 1) - dx_global_min).to_f64().unwrap(),
    };
    let spawn_span = (data.spawn.1 - data.spawn.0 + 1) as usize;
    for i in 0..880 {
        result.dist[i] = contrib[i..min(i + spawn_span, 880)].iter().sum();
    }
    return result;
}

fn calculate_regular(data: &ZombieData, ice_time: i64, time: i64) -> PosDistribution {
    let MovementType::Regular(anim_a, anim_b) = &data.movement_type else {
        unreachable!();
    };
    let dist_a = calculate_animation(data, ice_time, time, Some(anim_a));
    let dist_b = calculate_animation(data, ice_time, time, Some(anim_b));
    let mut result = PosDistribution {
        dist: [0.0; 880],
        min: f64::min(dist_a.min, dist_b.min),
        max: f64::max(dist_a.max, dist_b.max)
    };
    for i in 0..880 {
        result.dist[i] = (dist_a.dist[i] + dist_b.dist[i]) / 2.0;
    }
    return result
}

fn calculate_dancing(data: &ZombieData, ice_time: i64, time: i64) -> PosDistribution {
    let (norm_time, _) = calc_time(data, ice_time, time);
    if norm_time < 299 {
        return calculate_animation(data, 0, norm_time, None);
    }
    let maximum_norm_multiplier = max(310 - norm_time + 1, 1);
    let mut result = PosDistribution {
        dist: [0.0; 880],
        min: 1000.0,
        max: 0.0
    };
    for norm in 299..=min(norm_time, 310) {
        let d = calculate_animation(data, 0, norm, None);
        result.min = f64::min(result.min, d.min);
        result.max = f64::max(result.max, d.max);
        let multiplier = if norm == min(norm_time, 310) {maximum_norm_multiplier} else {1};
        for i in 0..880 {
            result.dist[i] += d.dist[i] * multiplier as f64 / 12.0;
        }
    }
    return result
}

fn calculate_zomboni(data: &ZombieData, _ice_time: i64, time: i64) -> PosDistribution {
    let mut result = PosDistribution {
        dist: [0.0; 880],
        min: 1000.0,
        max: 0.0
    };
    for spawn in data.spawn.0..=data.spawn.1 {
        let mut pos = spawn as f64;
        for _ in 0..time {
            pos -= ((pos - 700.0).floor() / 2000.0 + 0.25).clamp(0.1, 0.25);
        }
        result.dist[pos as usize] += 1.0 / (data.spawn.1 - data.spawn.0 + 1) as f64;
        result.min = f64::min(result.min, pos);
        result.max = f64::max(result.max, pos);
    }
    return result;
}

pub fn calculate_pos_distribution(data: &ZombieData, ice_time: i64, time: i64) -> PosDistribution {
    match data.movement_type {
        MovementType::Constant => calculate_constant(data, ice_time, time),
        MovementType::Animation(_) => calculate_animation(data, ice_time, time, None),
        MovementType::Regular(_, _) => calculate_regular(data, ice_time, time),
        MovementType::DanceCheat => calculate_dancecheat(data, ice_time, time),
        MovementType::Dancing(_) => calculate_dancing(data, ice_time, time),
        MovementType::Zomboni => calculate_zomboni(data, ice_time, time),
    }
}
