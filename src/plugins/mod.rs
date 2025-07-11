mod barista;
pub use self::barista::BaristaPlugin;

// mod bucket;
// pub use self::bucket::BucketPlugin;

mod chance;
pub use self::chance::ChancePlugin;

mod forecast;
pub use self::forecast::ForecastPlugin;

mod karma;
pub use self::karma::KarmaPlugin;

mod mention;
pub use self::mention::MentionPlugin;

mod net_tools;
pub use self::net_tools::NetToolsPlugin;

mod noaa;
pub use self::noaa::NoaaPlugin;

mod riddle;
pub use self::riddle::RiddlePlugin;

mod scryfall;
pub use self::scryfall::ScryfallPlugin;

mod introspection;
pub use self::introspection::IntrospectionPlugin;

mod help;
pub use self::help::HelpPlugin;
