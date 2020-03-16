use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    m.create_table("noaa", |t| {
        t.add_column("id", types::primary());
        t.add_column("nick", types::text().unique(true));
        t.add_column("station", types::text());
    });

    m.make::<Pg>()
}
