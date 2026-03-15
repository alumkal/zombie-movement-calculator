use crate::common::*;
use libm::erfc;
use num_traits::ToPrimitive;
use rayon::prelude::*;

#[cfg(debug_assertions)]
pub const MINMAX_ONLY_MODE: bool = true; // 调试模式
#[cfg(not(debug_assertions))]
pub const MINMAX_ONLY_MODE: bool = false;

#[derive(Clone, Copy)]
struct IceSegment {
    chill_time_max: i64,
    norm_time_after: i64,
    freeze_span: i64,
}

struct TimePlan {
    initial_norm_time: i64,
    segments: Vec<IceSegment>,
}

// 输入按升序排列的冰时机，按“初始原速 + 每次冰后减速段与原速段”构建时间分段。
fn calc_time(data: &ZombieData, ice_times: &[i64], time: i64) -> TimePlan {
    let mut normalized_ice_times = ice_times.to_vec();
    normalized_ice_times.sort_unstable();
    normalized_ice_times.dedup();
    let valid_ice_times: Vec<i64> = normalized_ice_times.into_iter().filter(|&t| (1..=time).contains(&t)).collect();
    if valid_ice_times.is_empty() || data.chill_immune {
        return TimePlan {initial_norm_time: time, segments: vec![],};
    }
    let initial_norm_time = max(valid_ice_times[0] - 1, 0);
    let mut segments: Vec<IceSegment> = Vec::with_capacity(valid_ice_times.len());
    let mut prev_primary_freeze = true;
    for i in 0..valid_ice_times.len() {
        let window_after_ice = 
            if i + 1 >= valid_ice_times.len() {time + 1} 
            else {valid_ice_times[i + 1]} - valid_ice_times[i];
        let next_primary_freeze = window_after_ice > 1999;
        let affected = if next_primary_freeze {1999} else {window_after_ice};
        let (chill_time_max, freeze_span) = 
            if data.freeze_immune {(affected, 1)} 
            else if prev_primary_freeze {(max(affected - 399, 0), 201)}
            else {(max(affected - 299, 0), 101)};
        let norm_time_after: i64 = if next_primary_freeze {window_after_ice - 1999} else {0};
        segments.push(IceSegment {chill_time_max, norm_time_after, freeze_span});
        prev_primary_freeze = next_primary_freeze;
    }
    TimePlan {initial_norm_time, segments,}
}

// 对于一个冰段，返回 chill_time 和对应权重的列表
fn get_segment_chill_states(seg: &IceSegment) -> Vec<(i64, Num)> {
    let minimum_chill_multiplier: i64 = max(seg.freeze_span - seg.chill_time_max, 1);
    let chill_time_min = max(seg.chill_time_max - seg.freeze_span + 1, 0);
    let mut result = Vec::new();
    for chill_time in chill_time_min..=seg.chill_time_max {
        if MINMAX_ONLY_MODE && chill_time != chill_time_min && chill_time != seg.chill_time_max {continue;}
        let chill_weight =
            if seg.freeze_span == 1 {Num::new(1, 1)}
            else {Num::new(if chill_time == chill_time_min {minimum_chill_multiplier} else {1}, seg.freeze_span)};
        result.push((chill_time, chill_weight));
    }
    result
}

// 将目前状态与一个冰段卷积，得到冰段结束后的状态列表
fn convolve_chill_states(base_states: &[(i64, Num)], seg: &IceSegment) -> Vec<(i64, Num)> {
    let seg_states = get_segment_chill_states(seg);
    let mut next_states: HashMap<i64, Num> = HashMap::new();
    for (base_sum, base_weight) in base_states {
        for (seg_sum, seg_weight) in &seg_states {
            let entry = next_states.entry(*base_sum + *seg_sum).or_insert(Num::new(0, 1));
            *entry += *base_weight * *seg_weight;
        }
    }
    next_states.into_iter().collect()
}

// 将初始状态与多个冰段卷积，得到所有可能的总冰时间与对应权重
fn get_total_chill_states(plan: &TimePlan) -> Vec<(i64, Num)> {
    let mut states: Vec<(i64, Num)> = vec![(0, Num::new(1, 1))];
    for seg in &plan.segments {states = convolve_chill_states(&states, seg);}
    states
}

fn calculate_constant(data: &ZombieData, ice_times: &[i64], time: i64) -> PosDistribution {
    let speed_min_norm = (data.speed.0 * 16384).round() / 16384;
    let speed_max_norm = (data.speed.1 * 16384).round() / 16384;
    let speed_min_chill = (data.speed.0 * Num::new(2, 5) * 16384).round() / 16384;
    let speed_max_chill = (data.speed.1 * Num::new(2, 5) * 16384).round() / 16384;
    let mut contrib = [0.0; 880];
    let plan = calc_time(data, ice_times, time);
    let norm_time_total = plan.initial_norm_time + plan.segments.iter().map(|x| x.norm_time_after).sum::<i64>();
    let chill_states = get_total_chill_states(&plan);
    let spawn_span = data.spawn.1 - data.spawn.0 + 1;
    let mut global_dx_min = Num::new(1000, 1);
    let mut global_dx_max = Num::new(0, 1);
    for (chill_time, state_weight) in chill_states {
        let weight = state_weight / spawn_span;
        let dx_min = speed_min_norm * norm_time_total + speed_min_chill * chill_time;
        let dx_max = speed_max_norm * norm_time_total + speed_max_chill * chill_time;
        global_dx_min = min(global_dx_min, dx_min);
        global_dx_max = max(global_dx_max, dx_max);
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
    else { (erfc(l / std::f64::consts::SQRT_2) - erfc(r / std::f64::consts::SQRT_2)) / 2.0 }
}

fn calculate_dancecheat(data: &ZombieData, ice_times: &[i64], time: i64) -> PosDistribution {
    let k = data.speed.0.to_f64().unwrap();
    let mut contrib = [0.0; 880];
    let plan = calc_time(data, ice_times, time);
    let norm_time_total = plan.initial_norm_time + plan.segments.iter().map(|x| x.norm_time_after).sum::<i64>();
    let chill_states = get_total_chill_states(&plan);
    let spawn_span = data.spawn.1 - data.spawn.0 + 1;
    for (chill_time, state_weight) in chill_states {
        let weight = state_weight.to_f64().unwrap() / spawn_span as f64;
        let norm_time = norm_time_total as f64;
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
        max: 880.0 - 1.0 / 16384.0 - dist.iter().rev().position(|&x| x > 1e-9).unwrap() as f64
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

// arr[x0] + arr[x0 + k] + ... + arr[x0 + (n - 1) * k]
fn total_shift(arr: &Vec<Num>, n: i64, k: Num, x0: Num) -> Num {
    let n = Num::new(n, 1);
    let mut result = Num::new(0, 1);
    let first = x0.floor().to_integer();
    let last = (x0 + (n - 1) * k).floor().to_integer();
    let mut cur = Num::new(0, 1);
    for i in first..last {
        let next = ((Num::new(i + 1, 1) - x0) / k).ceil();
        result += arr[i as usize % arr.len()] * (next - cur);
        cur = next;
    }
    return result + arr[last as usize % arr.len()] * (n - cur);
}

fn calculate_animation(data: &ZombieData, ice_times: &[i64], time: i64, animation: Option<&Vec<Num>>) -> PosDistribution {
    let animation = animation.unwrap_or_else(|| match &data.movement_type {
        MovementType::Animation(x) | MovementType::Dancing(x) => x,
        _ => unreachable!()
    });
    let anim_len = animation.len() as i64;
    let total: Num = animation.iter().sum();
    let speed_scale_factor = Num::new(47, 100) * anim_len / total;
    let dis_scale_factor = Num::new(anim_len + 1, anim_len);
    let plan = calc_time(data, ice_times, time);
    let n: i64 = plan.initial_norm_time * 2 + plan
        .segments
        .iter()
        .map(|x| x.chill_time_max + x.norm_time_after * 2)
        .sum::<i64>();
    // k 是减速状态下相位的变化率
    let k_min = data.speed.0 * speed_scale_factor / 2;
    let k_max = data.speed.1 * speed_scale_factor / 2;
    // k 在 [k_segments[i], k_segments[i+1]) 范围内变化时 dx 正比于 k
    let has_chill = plan.segments.iter().any(|x| x.chill_time_max > 0);
    let k_segments =
        if has_chill { fraction_between(n, k_min, k_max) }
        else { fraction_between(n / 2, k_min * 2, k_max * 2).iter().map(|x| x / 2).collect() };
    let (contrib, dx_global_min, dx_global_max) = k_segments
        .par_windows(2)
        .map(|lr| {
        let (l, r) = (lr[0], lr[1]);
        let mut contrib = [0.0; 880];
        let mut dx_global_min = Num::new(1000, 1);
        let mut dx_global_max = Num::new(0, 1);
        let mut shift_min = Num::new(0, 1);
        let mut shift_max = Num::new(0, 1);
        let mut phase = l * 2;
        let shift_norm_l: Vec<_> = animation.iter().map(|x| (x * dis_scale_factor * l * 32768).round() / 16384).collect();
        let shift_norm_r: Vec<_> = animation.iter().map(|x| (x * dis_scale_factor * r * 32768).round() / 16384).collect();
        let shift_l: Vec<_> = animation.iter().map(|x| (x * dis_scale_factor * l * 16384).round() / 16384).collect();
        let shift_r: Vec<_> = animation.iter().map(|x| (x * dis_scale_factor * r * 16384).round() / 16384).collect();

        shift_min += total_shift(&shift_norm_l, plan.initial_norm_time, l * 2, phase);
        shift_max += total_shift(&shift_norm_r, plan.initial_norm_time, l * 2, phase);
        phase += l * 2 * plan.initial_norm_time;

        let mut states: Vec<(Num, Num, Num, Num)> = vec![(phase, shift_min, shift_max, Num::new(1, 1))];
        let mut seg_index = 0usize;
        while seg_index < plan.segments.len() {
            let mut run_states: Vec<(i64, Num)> = vec![(0, Num::new(1, 1))];
            let mut tail_norm_after: i64 = 0;
            while seg_index < plan.segments.len() {
                let seg = &plan.segments[seg_index];
                run_states = convolve_chill_states(&run_states, seg);
                seg_index += 1;
                if seg.norm_time_after > 0 {
                    tail_norm_after = seg.norm_time_after;
                    break;
                }
            }
            run_states.sort_unstable_by_key(|(run_chill, _)| *run_chill);

            let mut next_states: Vec<(Num, Num, Num, Num)> = Vec::new();
            for (phase, shift_min, shift_max, weight) in states {
                if run_states.is_empty() {
                    continue;
                }
                let (chill_time_base, _) = run_states[0];
                let mut shift_min_cur = shift_min + total_shift(&shift_l, chill_time_base, l, phase);
                let mut shift_max_cur = shift_max + total_shift(&shift_r, chill_time_base, l, phase);
                let mut phase_cur = phase + l * chill_time_base;
                let mut prev_chill = chill_time_base;
                for (run_chill, run_weight) in &run_states {
                    let delta = *run_chill - prev_chill;
                    for _ in 0..delta {
                        shift_min_cur += shift_l[phase_cur.to_integer() as usize % animation.len()];
                        shift_max_cur += shift_r[phase_cur.to_integer() as usize % animation.len()];
                        phase_cur += l;
                    }
                    let shift_min_next = shift_min_cur + total_shift(&shift_norm_l, tail_norm_after, l * 2, phase_cur);
                    let shift_max_next = shift_max_cur + total_shift(&shift_norm_r, tail_norm_after, l * 2, phase_cur);
                    let phase_next = phase_cur + l * 2 * tail_norm_after;
                    next_states.push((phase_next, shift_min_next, shift_max_next, weight * *run_weight));
                    prev_chill = *run_chill;
                }
            }
            states = next_states;
        }
        let spawn_span = data.spawn.1 - data.spawn.0 + 1;
        let k_weight = 
            if k_min == k_max {Num::new(1, 1)} 
            else {(r - l) / (k_max - k_min)}; // avoid 0/0
        for (_, dx_min, dx_max, state_weight) in states {
            dx_global_min = min(dx_global_min, dx_min);
            dx_global_max = max(dx_global_max, dx_max);
            let weight = state_weight / spawn_span * k_weight;
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
                contrib[(data.spawn.1 - dx) as usize] += weight.to_f64().unwrap() * ratio.to_f64().unwrap();
            }
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

fn calculate_regular(data: &ZombieData, ice_times: &[i64], time: i64) -> PosDistribution {
    let MovementType::Regular(anim_a, anim_b) = &data.movement_type else {
        unreachable!();
    };
    let dist_a = calculate_animation(data, ice_times, time, Some(anim_a));
    let dist_b = calculate_animation(data, ice_times, time, Some(anim_b));
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

fn calculate_dancing(data: &ZombieData, ice_times: &[i64], time: i64) -> PosDistribution {
    let norm_time = calc_time(data, ice_times, time).initial_norm_time;
    if norm_time < 299 {
        return calculate_animation(data, &[0], norm_time, None);
    }
    let maximum_norm_multiplier = max(310 - norm_time + 1, 1);
    let mut result = PosDistribution {
        dist: [0.0; 880],
        min: 1000.0,
        max: 0.0
    };
    for norm in 299..=min(norm_time, 310) {
        let d = calculate_animation(data, &[0], norm, None);
        result.min = f64::min(result.min, d.min);
        result.max = f64::max(result.max, d.max);
        let multiplier = if norm == min(norm_time, 310) {maximum_norm_multiplier} else {1};
        for i in 0..880 {
            result.dist[i] += d.dist[i] * multiplier as f64 / 12.0;
        }
    }
    return result
}

fn calculate_zomboni(data: &ZombieData, _ice_times: &[i64], time: i64) -> PosDistribution {
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

pub fn calculate_pos_distribution(data: &ZombieData, ice_times: &[i64], time: i64) -> PosDistribution {
    match data.movement_type {
        MovementType::Constant => calculate_constant(data, ice_times, time),
        MovementType::Animation(_) => calculate_animation(data, ice_times, time, None),
        MovementType::Regular(_, _) => calculate_regular(data, ice_times, time),
        MovementType::DanceCheat => calculate_dancecheat(data, ice_times, time),
        MovementType::Dancing(_) => calculate_dancing(data, ice_times, time),
        MovementType::Zomboni => calculate_zomboni(data, ice_times, time),
    }
}
