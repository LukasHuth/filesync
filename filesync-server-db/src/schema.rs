// @generated automatically by Diesel CLI.

diesel::table! {
    file_blobs (file_hash, hash) {
        file_hash -> Text,
        hash -> Text,
    }
}

diesel::table! {
    files (hash) {
        hash -> Text,
        name -> Text,
        deleted -> Bool,
    }
}

diesel::joinable!(file_blobs -> files (file_hash));

diesel::allow_tables_to_appear_in_same_query!(file_blobs, files,);
