use std::fmt;
use std::vec::Vec;

use rand::seq::SliceRandom;
use rand::Rng;

// Some teas can only be in a certain set of containers. For instance, hohins
// will only hold some kind of green tea.
//
// Also, some adjectives only fit certain teas. example: Kangra => green tea,
// Irish => black tea.
//
// Some teas are only good at a certain temperature (cold butter tea is
// disgusting, to say the least).
//
// Because of that, we can't just choose a random tea, adjective, and vessel; we
// need to choose the tea first, then the vessel and adjective based on what tea
// was chosen.

// This is the chance that an adjective will preceed the tea.
const CHANCE_OF_TEA_ADJECTIVE: f64 = 0.25;

// This is the chance that a size is added to the vessel description.
const CHANCE_OF_VESSEL_SIZE: f64 = 0.90;

// This is the chance that an adjective will preceed the vessel.
const CHANCE_OF_VESSEL_ADJECTIVE: f64 = 0.85;

// Vessels for tea.
enum VesselType {
    Teapot,
    Mug,
    Bowl,
    Samovar,
    Teacup,
    Hohin,
    #[allow(dead_code)]
    Gaiwan,
    Shiboridashi,
}

impl VesselType {
    fn adjective_choices(&self) -> &[&str] {
        match *self {
            VesselType::Teapot => &[
                "vintage",
                "silver",
                "English",
                "antique silver",
                "jasperware",
            ],
            VesselType::Mug => &[
                "stoneware",
                "porcelain",
                "jasperware",
                "wooden",
                "Indian-made clay",
            ],
            VesselType::Bowl => &[
                "burl wood-and-silver tea",
                "Tibetan tea",
                "Tibetan silver tea",
            ],
            VesselType::Samovar => &["antique", "vintage", "brass", "silver"],
            VesselType::Teacup => &["porcelain"],
            VesselType::Hohin => &[],
            VesselType::Gaiwan => &["porcelain", "Ruyao"],
            VesselType::Shiboridashi => &["porcelain", "red clay"],
        }
    }
}

impl fmt::Display for VesselType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            VesselType::Teapot => f.write_str("teapot"),
            VesselType::Mug => f.write_str("mug"),
            VesselType::Bowl => f.write_str("bowl"),
            VesselType::Samovar => f.write_str("samovar"),
            VesselType::Teacup => f.write_str("teacup"),
            VesselType::Hohin => f.write_str("hohin"),
            VesselType::Gaiwan => f.write_str("gaiwan"),
            VesselType::Shiboridashi => f.write_str("shiboridashi"),
        }
    }
}

// Tea variants: brands, places, etc.
//
// In the previous implementation, these were called adjectives, but neither
// variant or adjective makes a ton of sense... especially because some of these
// are mutually exclusive (as an example, Earl Grey and English Breakfast), but
// some aren't (you could add Vanilla to pretty much any tea). It would be good
// to revisit later when the relevant variants are actually in use.
enum TeaVariant {
    Newmans,
    Earl,
    FairTrade,
    #[allow(dead_code)]
    Organic,
    Homemade,
    #[allow(dead_code)]
    HomeBrewn,
    // Kangra is a location in India where some kinds of green
    // tea are produced.
    Kangra,
    Irish,
    English,
    Darjeel,
    #[allow(dead_code)]
    Vanilla,
    #[allow(dead_code)]
    Lemongrass,
    #[allow(dead_code)]
    Hibiscus,
}

impl fmt::Display for TeaVariant {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TeaVariant::Newmans => f.write_str("Newman's Own"),
            TeaVariant::Earl => f.write_str("Earl Grey"),
            TeaVariant::FairTrade => f.write_str("fair trade"),
            TeaVariant::Organic => f.write_str("organic"),
            TeaVariant::Homemade => f.write_str("homemade"),
            TeaVariant::HomeBrewn => f.write_str("home-brewn"),
            TeaVariant::Kangra => f.write_str("Kangra"),
            TeaVariant::Irish => f.write_str("Irish-breakfast"),
            TeaVariant::English => f.write_str("English-breakfast"),
            TeaVariant::Darjeel => f.write_str("Darjeeling"),
            TeaVariant::Vanilla => f.write_str("vanilla"),
            TeaVariant::Lemongrass => f.write_str("lemongrass"),
            TeaVariant::Hibiscus => f.write_str("hibiscus"),
        }
    }
}

#[derive(Clone)]
enum Temperature {
    // Cold
    Iced,
    Cold,
    Chilled,
    IceCold,
    Freezing,
    // Warm
    Lukewarm,
    Warm,
    Warmish,
    RoomTemperature,
    // Hot
    Boiling,
    Scalding,
    Steaming,
    Sweltering,
    ToastyHot,
    RedHot,
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Temperature::Iced => f.write_str("iced"),
            Temperature::Cold => f.write_str("cold"),
            Temperature::Chilled => f.write_str("chilled"),
            Temperature::IceCold => f.write_str("ice-cold"),
            Temperature::Freezing => f.write_str("freezing"),
            Temperature::Lukewarm => f.write_str("lukewarm"),
            Temperature::Warm => f.write_str("warm"),
            Temperature::Warmish => f.write_str("warmish"),
            Temperature::RoomTemperature => f.write_str("room temperature"),
            Temperature::Boiling => f.write_str("boiling"),
            Temperature::Scalding => f.write_str("scalding"),
            Temperature::Steaming => f.write_str("steaming"),
            Temperature::Sweltering => f.write_str("sweltering"),
            Temperature::ToastyHot => f.write_str("toasty hot"),
            Temperature::RedHot => f.write_str("red-hot"),
        }
    }
}

// Temperature of tea.
//
// Note that we use lazy_static so we can compute COLD_HOT, WARM_HOT, and ALL,
// otherwise there's a ton of copying going on.
lazy_static::lazy_static! {
    static ref COLD: Vec<Temperature> = vec![
        Temperature::Iced,
        Temperature::Cold,
        Temperature::Chilled,
        Temperature::IceCold,
        Temperature::Freezing,
    ];
    static ref WARM: Vec<Temperature> = vec![
        Temperature::Lukewarm,
        Temperature::Warm,
        Temperature::Warmish,
        Temperature::RoomTemperature,
    ];
    static ref HOT: Vec<Temperature> = vec![
        Temperature::Boiling,
        Temperature::Scalding,
        Temperature::Steaming,
        Temperature::Sweltering,
        Temperature::ToastyHot,
        Temperature::RedHot,
    ];
    static ref COLD_HOT: Vec<Temperature> = COLD.iter().chain(HOT.iter()).cloned().collect();
    static ref WARM_HOT: Vec<Temperature> = WARM.iter().chain(HOT.iter()).cloned().collect();
    static ref ALL: Vec<Temperature> = COLD.iter().chain(WARM.iter()).chain(HOT.iter()).cloned().collect();
}

enum TeaType {
    Black,
    Green,
    MatchaGreen,
    SenchaGreen,
    White,
    Oolong,
    Puer,
    Chai,
    Butter,
    Christmas,
    Rooibos,
    Tulsi,
    LemonbalmTulsi,
    Spearmint,
    Peppermint,
    ChocolateMint,
    Mullein,
    LambsEars,
    TumericGinger,
    LemongrassVerbena,
    Lemongrass,
    BlackCurrantHibiscus,
    RoastedDandelionRoot,
    DandelionLeafAndRoot,
    Lavender,
    CinnamonApple,
}

impl TeaType {
    fn vessel_choices(&self) -> &[VesselType] {
        match *self {
            TeaType::Black => &[
                VesselType::Samovar,
                VesselType::Teapot,
                VesselType::Mug,
                VesselType::Teacup,
            ],
            TeaType::Green => &[VesselType::Teacup, VesselType::Mug, VesselType::Hohin],
            TeaType::MatchaGreen => &[
                VesselType::Hohin,
                VesselType::Teapot,
                VesselType::Mug,
                VesselType::Teacup,
            ],
            TeaType::SenchaGreen => &[
                VesselType::Shiboridashi,
                VesselType::Hohin,
                VesselType::Teapot,
                VesselType::Mug,
                VesselType::Teacup,
            ],
            TeaType::White => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::Oolong => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::Puer => &[
                VesselType::Shiboridashi,
                VesselType::Teapot,
                VesselType::Mug,
                VesselType::Teacup,
            ],
            TeaType::Chai => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::Butter => &[
                VesselType::Teapot,
                VesselType::Mug,
                VesselType::Bowl,
                VesselType::Teacup,
            ],
            TeaType::Christmas => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::Rooibos => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::Tulsi => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::LemonbalmTulsi => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::Spearmint => &[VesselType::Teapot, VesselType::Mug],
            TeaType::Peppermint => &[VesselType::Teapot, VesselType::Mug],
            TeaType::ChocolateMint => &[VesselType::Teapot, VesselType::Mug],
            TeaType::Mullein => &[VesselType::Mug],
            TeaType::LambsEars => &[VesselType::Mug],
            TeaType::TumericGinger => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::LemongrassVerbena => &[VesselType::Mug, VesselType::Teacup],
            TeaType::Lemongrass => &[VesselType::Mug, VesselType::Teacup],
            TeaType::BlackCurrantHibiscus => &[VesselType::Mug, VesselType::Teacup],
            TeaType::RoastedDandelionRoot => {
                &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup]
            }
            TeaType::DandelionLeafAndRoot => {
                &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup]
            }
            TeaType::Lavender => &[VesselType::Teapot, VesselType::Mug, VesselType::Teacup],
            TeaType::CinnamonApple => &[VesselType::Teapot, VesselType::Mug],
        }
    }

    fn variant_choices(&self) -> &[TeaVariant] {
        match *self {
            TeaType::Black => &[
                TeaVariant::Irish,
                TeaVariant::English,
                TeaVariant::Newmans,
                TeaVariant::Earl,
                TeaVariant::Darjeel,
            ],
            TeaType::Green => &[TeaVariant::Kangra],
            TeaType::MatchaGreen => &[],
            TeaType::SenchaGreen => &[],
            TeaType::White => &[TeaVariant::Earl],
            TeaType::Oolong => &[],
            TeaType::Puer => &[],
            TeaType::Chai => &[TeaVariant::Homemade],
            TeaType::Butter => &[TeaVariant::Homemade],
            TeaType::Christmas => &[TeaVariant::Homemade],
            TeaType::Rooibos => &[],
            TeaType::Tulsi => &[TeaVariant::FairTrade],
            TeaType::LemonbalmTulsi => &[TeaVariant::FairTrade],
            TeaType::Spearmint => &[TeaVariant::Homemade],
            TeaType::Peppermint => &[TeaVariant::Homemade],
            TeaType::ChocolateMint => &[TeaVariant::Homemade],
            TeaType::Mullein => &[TeaVariant::Homemade],
            TeaType::LambsEars => &[TeaVariant::Homemade],
            TeaType::TumericGinger => &[TeaVariant::Newmans],
            TeaType::LemongrassVerbena => &[TeaVariant::Homemade],
            TeaType::Lemongrass => &[TeaVariant::Homemade],
            TeaType::BlackCurrantHibiscus => &[],
            TeaType::RoastedDandelionRoot => &[TeaVariant::Homemade],
            TeaType::DandelionLeafAndRoot => &[TeaVariant::Homemade],
            TeaType::Lavender => &[TeaVariant::Homemade],
            TeaType::CinnamonApple => &[],
        }
    }

    fn heat_choices(&self) -> &[Temperature] {
        match *self {
            TeaType::Black => COLD_HOT.as_ref(),
            TeaType::Green => COLD_HOT.as_ref(),
            TeaType::MatchaGreen => COLD_HOT.as_ref(),
            TeaType::SenchaGreen => ALL.as_ref(),
            TeaType::White => ALL.as_ref(),
            TeaType::Oolong => WARM_HOT.as_ref(),
            TeaType::Puer => ALL.as_ref(),
            TeaType::Chai => WARM_HOT.as_ref(),
            TeaType::Butter => HOT.as_ref(),
            TeaType::Christmas => HOT.as_ref(),
            TeaType::Rooibos => ALL.as_ref(),
            TeaType::Tulsi => COLD_HOT.as_ref(),
            TeaType::LemonbalmTulsi => COLD_HOT.as_ref(),
            TeaType::Spearmint => COLD.as_ref(),
            TeaType::Peppermint => COLD.as_ref(),
            TeaType::ChocolateMint => COLD.as_ref(),
            TeaType::Mullein => ALL.as_ref(),
            TeaType::LambsEars => ALL.as_ref(),
            TeaType::TumericGinger => ALL.as_ref(),
            TeaType::LemongrassVerbena => COLD.as_ref(),
            TeaType::Lemongrass => COLD.as_ref(),
            TeaType::BlackCurrantHibiscus => ALL.as_ref(),
            TeaType::RoastedDandelionRoot => HOT.as_ref(),
            TeaType::DandelionLeafAndRoot => ALL.as_ref(),
            TeaType::Lavender => ALL.as_ref(),
            TeaType::CinnamonApple => ALL.as_ref(),
        }
    }
}

impl fmt::Display for TeaType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TeaType::Black => f.write_str("black tea"),
            TeaType::Green => f.write_str("green tea"),
            TeaType::MatchaGreen => f.write_str("matcha green tea"),
            TeaType::SenchaGreen => f.write_str("sencha green tea"),
            TeaType::White => f.write_str("white tea"),
            TeaType::Oolong => f.write_str("oolong tea"),
            TeaType::Puer => f.write_str("pu'er"),
            TeaType::Chai => f.write_str("chai"),
            TeaType::Butter => f.write_str("butter tea"),
            TeaType::Christmas => f.write_str("christmas tea"),
            TeaType::Rooibos => f.write_str("rooibos tea"),
            TeaType::Tulsi => f.write_str("tulsi tea"),
            TeaType::LemonbalmTulsi => f.write_str("lemonbalm and tulsi tea"),
            TeaType::Spearmint => f.write_str("spearmint tea"),
            TeaType::Peppermint => f.write_str("peppermint tea"),
            TeaType::ChocolateMint => f.write_str("chocolate mint tea"),
            TeaType::Mullein => f.write_str("mullein tea"),
            TeaType::LambsEars => f.write_str("lamb's ears tea"),
            TeaType::TumericGinger => f.write_str("tumeric ginger tea"),
            TeaType::LemongrassVerbena => f.write_str("lemongrass-verbena tea"),
            TeaType::Lemongrass => f.write_str("lemongrass tea"),
            TeaType::BlackCurrantHibiscus => f.write_str("black currant hibiscus tea"),
            TeaType::RoastedDandelionRoot => f.write_str("roasted dandelion root tea"),
            TeaType::DandelionLeafAndRoot => f.write_str("dandelion leaf-and-root tea"),
            TeaType::Lavender => f.write_str("lavender tea"),
            TeaType::CinnamonApple => f.write_str("cinnamon-apple tea"),
        }
    }
}

// We list all tea types rather than implementing a rand::Distribution because
// it matches with how we do all the other types and it's less error prone:
// using choose ensures we're doing this properly rather than generating an int
// in a range and mapping that to a TeaType. Additionally, because we're
// "constructing" enum variants here, the compiler will yell at us if we miss
// any of them.
const TEA_TYPES: &[TeaType] = &[
    TeaType::Black,
    TeaType::Green,
    TeaType::MatchaGreen,
    TeaType::SenchaGreen,
    TeaType::White,
    TeaType::Oolong,
    TeaType::Puer,
    TeaType::Chai,
    TeaType::Butter,
    TeaType::Christmas,
    TeaType::Rooibos,
    TeaType::Tulsi,
    TeaType::LemonbalmTulsi,
    TeaType::Spearmint,
    TeaType::Peppermint,
    TeaType::ChocolateMint,
    TeaType::Mullein,
    TeaType::LambsEars,
    TeaType::TumericGinger,
    TeaType::LemongrassVerbena,
    TeaType::Lemongrass,
    TeaType::BlackCurrantHibiscus,
    TeaType::RoastedDandelionRoot,
    TeaType::DandelionLeafAndRoot,
    TeaType::Lavender,
    TeaType::CinnamonApple,
];

const SIZES: &[&str] = &[
    "large", "small", "medium", "tall", "wide", "big", "100ml", "giant", "tiny",
];
const FILLED_WITH: &[&str] = &[
    "filled with",
    "of",
    "stuffed with",
    "full of",
    "brimming with",
];

// TODO: add function to look up a tea based on the name

pub(crate) fn prepare() -> String {
    let mut rng = rand::thread_rng();

    let filled_with = FILLED_WITH.choose(&mut rng).unwrap();

    let tea_type = TEA_TYPES.choose(&mut rng).unwrap();
    let tea_variants = tea_type.variant_choices();
    let heat = tea_type.heat_choices().choose(&mut rng).unwrap();

    let vessel_type = tea_type.vessel_choices().choose(&mut rng).unwrap();
    let vessel_adjs = vessel_type.adjective_choices();

    let vessel = if rng.gen_bool(CHANCE_OF_VESSEL_ADJECTIVE) && !vessel_adjs.is_empty() {
        if rng.gen_bool(CHANCE_OF_VESSEL_SIZE) {
            format!(
                "{} {} {}",
                SIZES.choose(&mut rng).unwrap(),
                vessel_adjs.choose(&mut rng).unwrap(),
                vessel_type
            )
        } else {
            format!("{} {}", vessel_adjs.choose(&mut rng).unwrap(), vessel_type)
        }
    } else {
        vessel_type.to_string()
    };

    let tea = if rng.gen_bool(CHANCE_OF_TEA_ADJECTIVE) && !tea_variants.is_empty() {
        format!("{} {}", tea_variants.choose(&mut rng).unwrap(), tea_type)
    } else {
        tea_type.to_string()
    };

    format!("{} {} {} {}", vessel, filled_with, heat, tea)
}
