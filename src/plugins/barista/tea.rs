use std::fmt;

use rand::seq::SliceRandom;
use rand::Rng;

// Some teas can only be in a certain set of containers. For instance, hohins
// will only hold some kind of green tea.
//
// Also, some adjectives only fit certain teas. example: Kangra => green tea,
// Irish => black tea.
//
// Some teas are only good at a certain temperature (cold butter tea is
// disgusting).
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
                "Indian-made clay mug",
            ],
            VesselType::Bowl => &[
                "burl wood-and-silver tea",
                "Tibetan tea",
                "Tibetan silver tea",
            ],
            VesselType::Samovar => &["antique", "vintage", "brass", "silver"],
            VesselType::Teacup => &[],
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

// Tea adjectives: brands, places, etc.
enum TeaAdjective {
    Newmans,
    Earl,
    FairTrade,
    Homemade,
    #[allow(dead_code)]
    HomeBrewn,
    // Kangra is a location in India where some kinds of green
    // tea are made.
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

impl fmt::Display for TeaAdjective {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            TeaAdjective::Newmans => f.write_str("Newman's Own"),
            TeaAdjective::Earl => f.write_str("Earl Grey"),
            TeaAdjective::FairTrade => f.write_str("fair trade"),
            TeaAdjective::Homemade => f.write_str("homemade"),
            TeaAdjective::HomeBrewn => f.write_str("home-brewn"),
            TeaAdjective::Kangra => f.write_str("Kangra"),
            TeaAdjective::Irish => f.write_str("Irish-breakfast"),
            TeaAdjective::English => f.write_str("English-breakfast"),
            TeaAdjective::Darjeel => f.write_str("Darjeeling"),
            TeaAdjective::Vanilla => f.write_str("vanilla"),
            TeaAdjective::Lemongrass => f.write_str("lemongrass"),
            TeaAdjective::Hibiscus => f.write_str("hibiscus"),
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
}

impl fmt::Display for Temperature {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            Temperature::Iced => f.write_str("iced"),
            Temperature::Cold => f.write_str("cold"),
            Temperature::Chilled => f.write_str("chilled"),
            Temperature::IceCold => f.write_str("ice cold"),
            Temperature::Lukewarm => f.write_str("lukewarm"),
            Temperature::Warm => f.write_str("warm"),
            Temperature::Warmish => f.write_str("warmish"),
            Temperature::RoomTemperature => f.write_str("room temperature"),
            Temperature::Boiling => f.write_str("boiling"),
            Temperature::Scalding => f.write_str("scalding"),
            Temperature::Steaming => f.write_str("steaming"),
            Temperature::Sweltering => f.write_str("sweltering"),
            Temperature::ToastyHot => f.write_str("toasty hot"),
        }
    }
}

// Temperature of tea.
const COLD: &[Temperature] = &[
    Temperature::Iced,
    Temperature::Cold,
    Temperature::Chilled,
    Temperature::IceCold,
];
static WARM: &[Temperature] = &[
    Temperature::Lukewarm,
    Temperature::Warm,
    Temperature::Warmish,
    Temperature::RoomTemperature,
];
static HOT: &[Temperature] = &[
    Temperature::Boiling,
    Temperature::Scalding,
    Temperature::Steaming,
    Temperature::Sweltering,
    Temperature::ToastyHot,
];

// Note that we use lazy_static so we can compute COLD_HOT, WARM_HOT, and ALL,
// otherwise there's a ton of copying going on.
lazy_static::lazy_static! {
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
        }
    }

    fn adjective_choices(&self) -> &[TeaAdjective] {
        match *self {
            TeaType::Black => &[
                TeaAdjective::Irish,
                TeaAdjective::English,
                TeaAdjective::Newmans,
                TeaAdjective::Earl,
                TeaAdjective::Darjeel,
            ],
            TeaType::Green => &[TeaAdjective::Kangra],
            TeaType::MatchaGreen => &[],
            TeaType::SenchaGreen => &[],
            TeaType::White => &[TeaAdjective::Earl],
            TeaType::Oolong => &[],
            TeaType::Puer => &[],
            TeaType::Chai => &[TeaAdjective::Homemade],
            TeaType::Butter => &[TeaAdjective::Homemade],
            TeaType::Christmas => &[TeaAdjective::Homemade],
            TeaType::Rooibos => &[],
            TeaType::Tulsi => &[TeaAdjective::FairTrade],
            TeaType::LemonbalmTulsi => &[TeaAdjective::FairTrade],
            TeaType::Spearmint => &[TeaAdjective::Homemade],
            TeaType::Peppermint => &[TeaAdjective::Homemade],
            TeaType::ChocolateMint => &[TeaAdjective::Homemade],
            TeaType::Mullein => &[TeaAdjective::Homemade],
            TeaType::LambsEars => &[TeaAdjective::Homemade],
            TeaType::TumericGinger => &[TeaAdjective::Newmans],
            TeaType::LemongrassVerbena => &[TeaAdjective::Homemade],
            TeaType::Lemongrass => &[TeaAdjective::Homemade],
            TeaType::BlackCurrantHibiscus => &[],
            TeaType::RoastedDandelionRoot => &[TeaAdjective::Homemade],
            TeaType::DandelionLeafAndRoot => &[TeaAdjective::Homemade],
            TeaType::Lavender => &[TeaAdjective::Homemade],
        }
    }

    fn heat_choices(&self) -> &[Temperature] {
        match *self {
            TeaType::Black => ALL.as_ref(),
            TeaType::Green => ALL.as_ref(),
            TeaType::MatchaGreen => ALL.as_ref(),
            TeaType::SenchaGreen => ALL.as_ref(),
            TeaType::White => ALL.as_ref(),
            TeaType::Oolong => WARM_HOT.as_ref(),
            TeaType::Puer => ALL.as_ref(),
            TeaType::Chai => WARM_HOT.as_ref(),
            TeaType::Butter => HOT.as_ref(),
            TeaType::Christmas => ALL.as_ref(),
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
            TeaType::RoastedDandelionRoot => ALL.as_ref(),
            TeaType::DandelionLeafAndRoot => ALL.as_ref(),
            TeaType::Lavender => ALL.as_ref(),
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


pub(crate) fn prepare() -> String {
    let mut rng = rand::thread_rng();

    let filled_with = FILLED_WITH.choose(&mut rng).unwrap();

    let tea_type = TEA_TYPES.choose(&mut rng).unwrap();
    let tea_adjs = tea_type.adjective_choices();
    let heat = tea_type.heat_choices().choose(&mut rng).unwrap();

    let vessel_type = tea_type.vessel_choices().choose(&mut rng).unwrap();
    let vessel_adjs = vessel_type.adjective_choices();

    let vessel = if rng.gen_bool(CHANCE_OF_VESSEL_ADJECTIVE) && vessel_adjs.len() > 0 {
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

    let tea = if rng.gen_bool(CHANCE_OF_TEA_ADJECTIVE) && tea_adjs.len() > 0 {
        format!("{} {}", tea_adjs.choose(&mut rng).unwrap(), tea_type)
    } else {
        tea_type.to_string()
    };

    format!("{} {} {} {}", vessel, filled_with, heat, tea)
}
