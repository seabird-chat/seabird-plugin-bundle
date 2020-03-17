use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    m.create_table("forecast_location", |t| {
        t.add_column("id", types::primary());
        t.add_column("nick", types::text().unique(true));
        t.add_column("address", types::text());
        t.add_column("lat", types::double());
        t.add_column("lng", types::double());
    });

    m.make::<Pg>()
}
