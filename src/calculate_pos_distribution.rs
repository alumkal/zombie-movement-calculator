use crate::common::*;
use num_traits::ToPrimitive;

fn calc_time(data: &ZombieData, ice_time: i32, time: i32) -> (i32, i32) {
    let orig_time = ice_time - 1; // 僵尸原速移动的时间
    if ice_time == 0 || ice_time > time || data.chill_immune { (time, 0) }
    else if data.freeze_immune { (orig_time + max(time - orig_time - 1999, 0), min(time - orig_time, 1999)) }
    else { (orig_time + max(time - orig_time - 1999, 0), max(min(time - orig_time, 1999) - 399, 0)) }
}

fn range_add_diff(arr: &mut [f64], l: i64, r: i64, value: Num) {
    let value = value.to_f64().unwrap();
    arr[l as usize] += value;
    if r < 880 {
        arr[r as usize] -= value;
    }
}

fn calculate_constant(data: &ZombieData, ice_time: i32, time: i32) -> [f64; 880] {
    let chill_factor = match data.movement_type {
        MovementType::DanceCheat => Num::new(1, 2),
        _ => Num::new(2, 5)
    };
    let mut contrib_diff = [0.0; 880];
    let (orig_time, chill_time_max) = calc_time(data, ice_time, time);
    let chill_time_min = if data.freeze_immune {chill_time_max} else {max(chill_time_max - 200, 0)};
    let minimum_chill_multiplier = 201 - (chill_time_max - chill_time_min) as i64;
    let spawn_span = (data.spawn.1 - data.spawn.0 + 1) as i64;
    for chill in chill_time_min..=chill_time_max {
        let n = Num::new(chill.into(), 1) * chill_factor + orig_time as i64;
        let speed_span = data.speed.1 - data.speed.0;
        let weight = Num::new(if chill == chill_time_min {minimum_chill_multiplier} else {1}, 201)
        / spawn_span / (if speed_span == Num::new(0, 1) {Num::new(1, 1)} else {speed_span});
        let shift_min = data.speed.0 * n;
        let shift_max = data.speed.1 * n;
        let speed_per_px = Num::new(1, 1) / n;
        let border_l = (shift_min.ceil() - shift_min) * speed_per_px;
        let border_r = (shift_max - shift_max.floor()) * speed_per_px;
        let pos_min = data.spawn.0 as i64 - shift_max.ceil().to_integer();
        let pos_max = data.spawn.0 as i64 - shift_min.ceil().to_integer();
        if speed_span == Num::new(0, 1) {
            range_add_diff(&mut contrib_diff, pos_min, pos_max + 1, weight);
        } else {
            range_add_diff(&mut contrib_diff, pos_min, pos_min + 1, border_r * weight);
            range_add_diff(&mut contrib_diff, pos_min + 1, pos_max, speed_per_px * weight);
            range_add_diff(&mut contrib_diff, pos_max, pos_max + 1, border_l * weight);
        }
    }
    for i in 1..880 {
        contrib_diff[i] += contrib_diff[i - 1];
    }
    for i in 1..880 {
        contrib_diff[i] += contrib_diff[i - 1];
    }
    let mut result = [0.0; 880];
    for i in 0..880 {
        result[i] = if (i as i64) - spawn_span < 0 {
            contrib_diff[i]
        } else {
            contrib_diff[i] - contrib_diff[i - spawn_span as usize]
        };
    }
    return result;
}

// 返回分母<=n且在(l, r)之间的所有分数，外加l和r
fn fraction_between(n: i32, l: Num, r: Num) -> Vec<Num> {
    let mut result = vec![l];
    for i in 1..n as i64 {
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

fn calculate_animation(data: &ZombieData, ice_time: i32, time: i32, animation: Option<&Vec<Num>>) -> [f64; 880] {
    let animation = animation.unwrap_or(match &data.movement_type {
        MovementType::Animation(x) => x,
        MovementType::Dancing(x) => x,
        MovementType::Regular(x, _) => x,
        _ => unreachable!()
    });
    let total: Num = animation.iter().sum();
    let speed_scale_factor = Num::new(47, 100) * animation.len() as i64 / total;
    // 原速 orig_time cs, 减速 [chill_time_max-200, chill_time_max] cs
    let (orig_time, chill_time_max) = calc_time(data, ice_time, time);
    let n = orig_time * 2 + chill_time_max;
    // k是减速状态下相位的前进速率
    let k_min = data.speed.0 * speed_scale_factor / 2;
    let k_max = data.speed.1 * speed_scale_factor / 2;
    // k在[k_segments[i], k_segments[i+1])变化时dx正比于k
    let k_segments = fraction_between(n, k_min, k_max);

    let dis_scale_factor = Num::new((animation.len() + 1) as i64, animation.len() as i64);
    // 举例：chill_time = 100时，实际减速时间取到最小值0的概率是101/201(冰500-600cs)，是取到其他数值的101倍
    let minimum_chill_multiplier = max(200 - chill_time_max + 1, 1) as i64;
    // 支持[l, r)区间加的差分数组
    let mut result_diff = [0.0; 880];
    for i in 0..(k_segments.len() - 1) {
        let l = k_segments[i];
        let r = k_segments[i + 1];
        // shift = dx / k
        let mut shift = Num::new(0, 1);
        let mut phase = l * 2;
        for _ in 0..orig_time {
            shift += animation[phase.to_integer() as usize % animation.len()] * dis_scale_factor * 2;
            phase += l * 2;
        }
        for _ in 0..max(chill_time_max - 200, 0) {
            shift += animation[phase.to_integer() as usize % animation.len()] * dis_scale_factor;
            phase += l;
        }
        for i in 0..=min(chill_time_max, 200) {
            // 逐个更新不同冻结时间的期望
            let spawn_span = (data.spawn.1 - data.spawn.0 + 1) as i64;
            let weight = Num::new(if i == 0 {minimum_chill_multiplier} else {1}, 201)
                / spawn_span / (if k_min == k_max {Num::new(1, 1)} else {k_max - k_min}); // avoid 0/0
            let dx_l = (shift * l).ceil().to_integer();
            let dx_r = (shift * r).ceil().to_integer();
            for dx in dx_l..=dx_r {
                let pos = data.spawn.0 as i64 - dx;
                let speed_range =
                    if k_min == k_max {Num::new(1, 1)}
                    else if shift == 0.into() {r - l}
                    else {
                        let speed_l = max(Num::new(dx - 1, 1) / shift, l);
                        let speed_r = min(Num::new(dx, 1) / shift, r);
                        speed_r - speed_l
                    };
                range_add_diff(&mut result_diff, pos, pos + spawn_span, weight * speed_range);
            }
            shift += animation[phase.to_integer() as usize % animation.len()] * dis_scale_factor;
            phase += l;
        }
    }
    for i in 1..880 {
        result_diff[i] += result_diff[i - 1];
    }
    return result_diff;
}

fn calculate_regular(data: &ZombieData, ice_time: i32, time: i32) -> [f64; 880]  {
    let (anim_a, anim_b) = match &data.movement_type {
        MovementType::Regular(a, b) => (a, b),
        _ => unreachable!()
    };
    let result_a = calculate_animation(data, ice_time, time, Some(anim_a));
    let result_b = calculate_animation(data, ice_time, time, Some(anim_b));
    let mut result = [0.0; 880];
    for i in 0..880 {
        result[i] = (result_a[i] + result_b[i]) / 2.0;
    }
    return result;
}

///! This is just one case; actually dancing zombie will behave differently depending on other operations
fn calculate_dancing(data: &ZombieData, ice_time: i32, time: i32) -> [f64; 880] {
    let (orig_time, _) = calc_time(data, ice_time, time);
    if orig_time < 299 {
        return calculate_animation(data, 0, orig_time, None);
    }
    let maximum_orig_multiplier = max(310 - orig_time + 1, 1);
    let mut result = [0.0; 880];
    for orig in 299..=min(orig_time, 310) {
        let x = calculate_animation(data, 0, orig, None);
        let multiplier = if orig == max(orig_time, 310) {maximum_orig_multiplier} else {1};
        for i in 0..880 {
            result[i] += x[i] * multiplier as f64 / 12.0;
        }
    }
    return result;
}

fn calculate_zomboni(data: &ZombieData, _ice_time: i32, time: i32) -> [f64; 880] {
    let mut result = [0.0; 880];
    for spawn in data.spawn.0..=data.spawn.1 {
        let mut pos = spawn as f64;
        for _ in 0..time {
            pos -= ((pos - 700.0).floor() / 2000.0 + 0.25).clamp(0.1, 0.25);
        }
        result[pos as usize] += 1.0 / (data.spawn.1 - data.spawn.0 + 1) as f64;
    }
    return result;
}

pub fn calculate_pos_distribution(data: &ZombieData, ice_time: i32, time: i32) -> [f64; 880] {
    match data.movement_type {
        MovementType::Constant => calculate_constant(data, ice_time, time),
        MovementType::Animation(_) => calculate_animation(data, ice_time, time, None),
        MovementType::Regular(_, _) => calculate_regular(data, ice_time, time),
        MovementType::DanceCheat => calculate_constant(data, ice_time, time),
        MovementType::Dancing(_) => calculate_dancing(data, ice_time, time),
        MovementType::Zomboni => calculate_zomboni(data, ice_time, time),
    }
}
