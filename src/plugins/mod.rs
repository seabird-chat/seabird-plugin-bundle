// mod bucket;
// pub use self::bucket::BucketPlugin;

mod forecast;
pub use self::forecast::ForecastPlugin;

mod karma;
pub use self::karma::KarmaPlugin;

mod mention;
pub use self::mention::MentionPlugin;

mod minecraft;
pub use self::minecraft::MinecraftPlugin;

mod net_tools;
pub use self::net_tools::NetToolsPlugin;

mod noaa;
pub use self::noaa::NoaaPlugin;

mod introspection;
pub use self::introspection::IntrospectionPlugin;

mod url;
pub use self::url::UrlPlugin;
