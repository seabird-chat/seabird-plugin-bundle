use rand::seq::SliceRandom;

const POPS: &[&str] = &[
    "Coke",
    "Diet Coke",
    "Coke Zero",
    "Pepsi",
    "Diet Pepsi",
    "Mountain Dew",
    "Sun Drop",
];
const POP_SIZES: &[&str] = &[
    "small",
    "medium",
    "large",
    "extra large",
    "CostCo sized",
    "12 oz",
    "2 L",
];
const POP_STYLES: &[&str] = &["sizzling", "bubbling", "flat"];
const POP_HEATS: &[&str] = &["iced", "cold", "lukewarm", "room temp"];

pub(crate) fn prepare() -> String {
    let mut rng = rand::thread_rng();

    let size = POP_SIZES.choose(&mut rng).unwrap();
    let heat = POP_HEATS.choose(&mut rng).unwrap();
    let style = POP_STYLES.choose(&mut rng).unwrap();
    let pop = POPS.choose(&mut rng).unwrap();

    format!("{} {} {} {}", size, style, heat, pop)
}
