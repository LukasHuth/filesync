use diesel::{QueryDsl as _, RunQueryDsl as _, SelectableHelper as _, dsl::not};

mod connect;
mod schema;

pub mod dto;
pub fn get_available_files() -> Result<Vec<dto::AvailableFileDTO>, ()> {
    let mut conn = connect::get_connection().map_err(|_| ())?;
    schema::files::dsl::files
        .select(dto::AvailableFileDTO::as_select())
        .filter(not(schema::files::deleted))
        .get_results::<dto::AvailableFileDTO>(&mut conn)
        .map_err(|_| ())
}
pub fn register_file(name: String, hash: String) -> Result<usize, ()> {
    let mut conn = connect::get_connection().map_err(|_| ())?;
    diesel::insert_into(schema::files::dsl::files)
        .values(dto::AvailableFileDTO {
            name,
            hash,
            deleted: false,
        })
        .execute(&mut conn)
        .map_err(|_| ())
}
