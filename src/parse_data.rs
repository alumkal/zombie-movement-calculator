use crate::common::*;

#[derive(serde::Deserialize)]
struct RawZombieData {
    name: String,
    spawn_l: i32,
    spawn_r: i32,
    spawn_hugewave_l: i32,
    spawn_hugewave_r: i32,
    movement_type: String,
    movement_args: Option<String>,
    speed_l: String,
    speed_r: String,
    freeze_immune: bool,
    chill_immune: bool,
    defx_l: i32,
    defx_r: i32,
    defy_l: i32,
    defy_r: i32,
    atk_l: i32,
    atk_r: i32,
    hp: i32,
    summon_weight_normal: u32,
    summon_weight_hugewave: u32,
    on_ground: bool,
    in_pool: bool,
    in_pool_wave1to5: bool
}

fn decimal_to_rational(decimal: &str) -> Num {
    return match decimal.find('.') {
        Some(pos) => {
            let negative = decimal.starts_with('-');
            let int_part = decimal[(negative as usize)..pos].parse::<i64>().unwrap();
            let frac_part = decimal[pos+1..].parse::<i64>().unwrap();
            let den = i64::pow(10, (decimal.len() - pos - 1) as u32);
            let num = (if negative {-1} else {1}) * (int_part * den + frac_part);
            Num::new(num, den)
        },
        None => decimal.parse::<Num>().unwrap()
    };
}

fn parse_animation(raw: &str) -> Vec<Num> {
    let nums = raw.split(',').map(decimal_to_rational).collect::<Vec<_>>();
    return nums[1..].iter()
                    .zip(nums[..nums.len() - 1].iter())
                    .map(|(x, y)| x - y)
                    .collect::<Vec<_>>();
}

fn parse_regular(raw: &str) -> (Vec<Num>, Vec<Num>) {
    let sep = raw.find(';').unwrap();
    return (parse_animation(&raw[..sep]), parse_animation(&raw[sep + 1..]));
}

fn convert_zombie_data(data: RawZombieData) -> ZombieData {
    let movement_type = match data.movement_type.as_str() {
        "constant" => MovementType::Constant,
        "animation" => MovementType::Animation(parse_animation(&data.movement_args.unwrap())),
        "regular" => {
            let tmp = parse_regular(&data.movement_args.unwrap());
            MovementType::Regular(tmp.0, tmp.1)
        },
        "dancecheat" => MovementType::DanceCheat,
        "dancing" => MovementType::Dancing(parse_animation(&data.movement_args.unwrap())),
        "zomboni" => MovementType::Zomboni,
        _ => unreachable!()
    };
    return ZombieData {
        spawn: (data.spawn_l, data.spawn_r),
        spawn_hugewave: (data.spawn_hugewave_l, data.spawn_hugewave_r),
        movement_type: movement_type,
        speed: (decimal_to_rational(&data.speed_l), decimal_to_rational(&data.speed_r)),
        freeze_immune: data.freeze_immune,
        chill_immune: data.chill_immune,
        def_x: (data.defx_l, data.defx_r),
        def_y: (data.defy_l, data.defy_r),
        atk: (data.atk_l, data.atk_r),
        hp: data.hp,
        summon_weight_normal: data.summon_weight_normal,
        summon_weight_hugewave: data.summon_weight_hugewave,
        if_generate_in: (data.on_ground, data.in_pool),
        if_generate_in_wave1to5: (data.on_ground, data.in_pool_wave1to5)
    };
}

pub fn get_zombie_db(file_content: &[u8]) -> HashMap<ZombieType, ZombieData> {
    let mut csv_reader = csv::Reader::from_reader(file_content);
    return csv_reader.deserialize()
                     .map(|x: Result<RawZombieData, _>| x.unwrap())
                     .map(|x| (ZombieType::from_str(&x.name).expect(&x.name), convert_zombie_data(x)))
                     .collect();
}
