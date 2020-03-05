use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    m.create_table("karma", |t| {
        t.add_column("name", types::text().primary(true));
        t.add_column("score", types::integer().default(0).nullable(false));
    });

    m.make::<Pg>()
}
