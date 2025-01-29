use rand::seq::SliceRandom;

const POPS: &[&str] = &[
    // Coke and variants
    "Coke",
    "Diet Coke",
    "Coke Zero",
    "Cherry Coke",
    "Vanilla Coke",
    // Pepsi and variants
    "Pepsi",
    "Diet Pepsi",
    "Wild Cherry Pepsi",
    // Ginger Ales
    "Canada Dry",
    "Vernors",
    // Root Beers
    "IBC Root Beer",
    "A&W Root Beer",
    "Mug Root Beer",
    "Barq's Root Beer",
    // Jones
    "Jones Cream Soda",
    "Jones Berry Lemonade",
    "Jones Strawberry Lime",
    "Jones Green Apple",
    // Faygo
    "Faygo Red Pop",
    "Faygo Cream Soda",
    "Faygo Rock & Rye",
    "Faygo Grape",
    "Faygo Root Beer",
    // Other
    "7 Up",
    "Dr. Pepper",
    "Jones Cream Soda",
    "Mello Yello",
    "Mountain Dew Baja Blast",
    "Mountain Dew",
    "Orange Crush",
    "Orange Fanta",
    "RC Cola",
    "Starry",
    "Sprite",
    "Squirt",
    "Sun Drop",
    "Surge",
];
const POP_SIZES: &[&str] = &[
    "small",
    "medium",
    "large",
    "extra large",
    "CostCo sized",
    "12 oz",
    "128 oz",
    "2 L",
    "child sized",
    "kiddie sized",
];
const POP_STYLES: &[&str] = &["sizzling", "bubbling", "flat"];
const POP_HEATS: &[&str] = &["iced", "cold", "lukewarm", "room temp", "frozen"];

pub(crate) fn prepare() -> String {
    let mut rng = rand::thread_rng();

    let size = POP_SIZES.choose(&mut rng).unwrap();
    let heat = POP_HEATS.choose(&mut rng).unwrap();
    let style = POP_STYLES.choose(&mut rng).unwrap();
    let pop = POPS.choose(&mut rng).unwrap();

    format!("{} {} {} {}", size, style, heat, pop)
}
