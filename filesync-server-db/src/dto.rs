use diesel::{Selectable, deserialize::Queryable, prelude::Insertable};

#[derive(Selectable, Queryable, Insertable)]
#[diesel(table_name = crate::schema::files)]
#[diesel(check_for_backend(diesel::sqlite::Sqlite))]
pub struct AvailableFileDTO {
    pub name: String,
    pub hash: String,
    pub deleted: bool,
}
