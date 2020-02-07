use diesel::sql_types;

no_arg_sql_function!(
    random,
    sql_types::Integer,
    "Represents the SQL RANDOM() function"
);
