table! {
    bucket_facts (id) {
        id -> Int4,
        fact -> Text,
        verb -> Text,
        tidbit -> Text,
    }
}

table! {
    karma (name) {
        name -> Text,
        score -> Int4,
    }
}

allow_tables_to_appear_in_same_query!(
    bucket_facts,
    karma,
);
