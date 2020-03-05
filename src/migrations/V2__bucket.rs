use barrel::{backend::Pg, types, Migration};

pub fn migration() -> String {
    let mut m = Migration::new();

    m.create_table("bucket_facts", |t| {
        t.add_column("id", types::primary());
        t.add_column("fact", types::text());
        t.add_column("verb", types::text().default("is"));
        t.add_column("tidbit", types::text());

        t.add_index(
            "bucket_facts_lookup",
            types::index(vec!["fact", "verb", "tidbit"]).unique(true),
        );
    });

    m.make::<Pg>()
}
