use rand::seq::SliceRandom;
use rand::Rng;

const COFFEES: &[&str] = &[
    "mocha",
    "latte",
    "macchiato",
    "breve",
    "americano",
    "cubano",
    "cappuccino",
];
const COFFEE_SIZES: &[&str] = &[
    "small", "medium", "short", "tall", "large", "grande", "venti",
];
const COFFEE_FLAVORS: &[&str] = &[
    "hazelnut",
    "white chocolate",
    "dark chocolate",
    "caramel",
    "vanilla",
    "cinnamon",
];
const COFFEE_HEATS: &[&str] = &[
    "iced",
    "cold",
    "lukewarm",
    "warm",
    "hot",
    "boiling hot",
    "scalding",
];

pub(crate) fn prepare() -> String {
    let mut rng = rand::thread_rng();

    let size = COFFEE_SIZES.choose(&mut rng).unwrap();
    let heat = COFFEE_HEATS.choose(&mut rng).unwrap();
    let flavor = COFFEE_FLAVORS.choose(&mut rng).unwrap();
    let coffee = COFFEES.choose(&mut rng).unwrap();

    let shots: u32 = rng.gen_range(1..=3);
    let shots_str = if shots == 1 { "shot" } else { "shots" };

    format!(
        "{} {} {} {} with {} {} of espresso",
        size, heat, flavor, coffee, shots, shots_str
    )
}
